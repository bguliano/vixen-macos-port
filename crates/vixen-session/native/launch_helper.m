// An action owns this helper, its private Unix socket, and only the process it
// creates. No automation scripts or process-name searches stop applications.
#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>
#include <crt_externs.h>
#include <errno.h>
#include <fcntl.h>
#include <libproc.h>
#include <poll.h>
#include <signal.h>
#include <spawn.h>
#include <sys/proc_info.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <unistd.h>

static BOOL write_all(int fd, const void *bytes, size_t count) {
    const char *p = bytes;
    while (count) { ssize_t n = write(fd, p, count); if (n < 0 && errno == EINTR) continue; if (n <= 0) return NO; p += n; count -= (size_t)n; }
    return YES;
}
static BOOL event(int fd, NSString *name, NSString *error) {
    NSDictionary *value = error ? @{ @"event": name, @"error": error } : @{ @"event": name };
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:nil];
    return write_all(fd, data.bytes, data.length) && write_all(fd, "\n", 1);
}
static BOOL stopping(int fd) {
    struct pollfd p = { .fd = fd, .events = POLLIN };
    if (poll(&p, 1, 0) <= 0) return NO;
    char bytes[32];
    // Only START is sent before launch; any later command or lost owner stops it.
    return read(fd, bytes, sizeof(bytes)) <= 0 || p.revents != 0;
}
static NSString *run_program(int fd, NSString *program, NSArray<NSString *> *arguments) {
    NSUInteger count = arguments.count + 2;
    char **argv = calloc(count, sizeof(char *));
    argv[0] = strdup(program.fileSystemRepresentation);
    for (NSUInteger i = 0; i < arguments.count; ++i) argv[i + 1] = strdup(arguments[i].UTF8String);
    posix_spawnattr_t attr; posix_spawnattr_init(&attr);
    posix_spawnattr_setflags(&attr, POSIX_SPAWN_SETPGROUP);
    posix_spawnattr_setpgroup(&attr, 0); // a new group whose id is the owned child
    pid_t child = 0;
    int error = posix_spawnp(&child, argv[0], NULL, &attr, argv, *_NSGetEnviron());
    posix_spawnattr_destroy(&attr);
    for (NSUInteger i = 0; i + 1 < count; ++i) free(argv[i]); free(argv);
    if (error) return [NSString stringWithUTF8String:strerror(error)];
    BOOL stop = !event(fd, @"started", nil);
    int status;
    for (;;) {
        if (stop || stopping(fd)) {
            // The unreaped direct child pins this process-group id against reuse.
            kill(-child, SIGKILL);
            kill(child, SIGKILL); // the direct child may have moved process groups
            while (waitpid(child, &status, 0) < 0 && errno == EINTR) {}
            return nil;
        }
        pid_t ended = waitpid(child, &status, WNOHANG);
        if (ended == child) return nil;
        if (ended < 0 && errno != EINTR) return [NSString stringWithUTF8String:strerror(errno)];
        usleep(20000);
    }
}
static BOOL created_after(pid_t pid, struct timeval before) {
    struct proc_bsdinfo info;
    if (proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof(info)) != sizeof(info)) return NO;
    return info.pbi_start_tvsec > (uint64_t)before.tv_sec ||
        (info.pbi_start_tvsec == (uint64_t)before.tv_sec && info.pbi_start_tvusec >= (uint64_t)before.tv_usec);
}
static NSString *run_application(int fd, NSString *program, NSArray<NSString *> *arguments, NSDictionary *request) {
    NSURL *url = [[NSURL fileURLWithPath:program] URLByResolvingSymlinksInPath];
    NSWorkspace *workspace = NSWorkspace.sharedWorkspace;
    NSMutableSet<NSNumber *> *existing = [NSMutableSet set];
    for (NSRunningApplication *running in workspace.runningApplications) [existing addObject:@(running.processIdentifier)];
    NSWorkspaceOpenConfiguration *configuration = NSWorkspaceOpenConfiguration.configuration;
    configuration.arguments = arguments;
    configuration.createsNewApplicationInstance = YES;
    configuration.allowsRunningApplicationSubstitution = NO;
    configuration.promptsUserIfNeeded = NO;
    configuration.addsToRecentItems = NO;
    NSObject *lock = [NSObject new];
    __block NSRunningApplication *application = nil;
    __block NSError *launchError = nil;
    __block BOOL complete = NO;
    struct timeval before; gettimeofday(&before, NULL);
    void (^launch)(void) = ^{
        [workspace openApplicationAtURL:url configuration:configuration completionHandler:^(NSRunningApplication *app, NSError *error) {
            @synchronized(lock) { application = app; launchError = error; complete = YES; }
        }];
    };
    NSTimeInterval timeout = 30;
#ifdef VIXEN_LAUNCH_TESTING
    // Only a separately compiled test helper contains this deterministic seam.
    // It models Launch Services creating an app after caller timeout/cancel.
    NSTimeInterval delay = [request[@"testLaunchDelayMs"] doubleValue] / 1000;
    if (request[@"testTimeoutMs"]) timeout = [request[@"testTimeoutMs"] doubleValue] / 1000;
    if (delay > 0) dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(delay * NSEC_PER_SEC)), dispatch_get_main_queue(), launch);
    else launch();
#else
    (void)request;
    launch();
#endif
    BOOL stop = NO, owned = NO;
    BOOL reportedPending = NO;
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:timeout];
    for (;;) {
        stop = stop || stopping(fd);
        NSRunningApplication *app; NSError *error; BOOL finished;
        @synchronized(lock) { app = application; error = launchError; finished = complete; }
        if (finished) {
            if (!app) return error.localizedDescription ?: @"Application could not be launched";
            if (app.terminated) return nil;
            if (!owned) {
                // Launch Services can reuse an instance despite a new-instance request.
                // Refuse ownership rather than ever terminating somebody else's app.
                if ([existing containsObject:@(app.processIdentifier)] || !created_after(app.processIdentifier, before))
                    return @"Launch Services returned a preexisting application; it was left running";
                owned = YES;
#ifdef VIXEN_LAUNCH_TESTING
                NSString *observed = request[@"testObservedPid"];
                if ([observed isKindOfClass:NSString.class])
                    [[NSString stringWithFormat:@"%d\n", app.processIdentifier] writeToFile:observed atomically:YES encoding:NSUTF8StringEncoding error:nil];
#endif
                if (!event(fd, @"started", nil)) stop = YES;
            }
            if (stop) {
                if (![app forceTerminate]) return @"Could not stop the launched application";
                NSDate *end = [NSDate dateWithTimeIntervalSinceNow:5];
                while (!app.terminated && end.timeIntervalSinceNow > 0)
                    [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
                return app.terminated ? nil : @"The launched application did not terminate";
            }
        } else if (!reportedPending && deadline.timeIntervalSinceNow <= 0) {
            // NSWorkspace has no cancellation API. Bound caller completion, not
            // this guard's lifetime: a late callback must still stop its own app.
            stop = YES;
            reportedPending = YES;
            event(fd, @"pending", @"Application launch is still pending; cancellation will stop any owned instance when Launch Services returns");
        }
        [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
        usleep(5000);
    }
}
static void cleanup(NSString *directory) {
    if (![directory.lastPathComponent hasPrefix:@"vixen-launch-"]) return;
    int fd = open(directory.fileSystemRepresentation, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
    if (fd < 0) return;
    struct stat info;
    if (fstat(fd, &info) == 0 && info.st_uid == geteuid() && (info.st_mode & 0777) == 0700) {
        // Only this protocol's files; never remove arbitrary directory content.
        // Also runs when the owner exited before its reaper could clean up.
        unlinkat(fd, "request.json", 0);
        unlinkat(fd, "control.sock", 0);
        unlinkat(fd, "launch.command", 0);
        rmdir(directory.fileSystemRepresentation);
    }
    close(fd);
}
static int run(NSString *directory) {
    @autoreleasepool {
        signal(SIGPIPE, SIG_IGN);
        NSData *data = [NSData dataWithContentsOfFile:[directory stringByAppendingPathComponent:@"request.json"]];
        NSDictionary *request = data ? [NSJSONSerialization JSONObjectWithData:data options:0 error:nil] : nil;
        if (![request isKindOfClass:NSDictionary.class] || ![request[@"program"] isKindOfClass:NSString.class] || ![request[@"arguments"] isKindOfClass:NSArray.class] || ![request[@"token"] isKindOfClass:NSString.class]) return 2;
        for (id value in request[@"arguments"]) if (![value isKindOfClass:NSString.class]) return 2;
        NSString *path = [directory stringByAppendingPathComponent:@"control.sock"];
        struct sockaddr_un address = { .sun_family = AF_UNIX };
        if (strlen(path.fileSystemRepresentation) >= sizeof(address.sun_path)) return 2;
        strcpy(address.sun_path, path.fileSystemRepresentation);
        int fd = socket(AF_UNIX, SOCK_STREAM, 0);
        if (fd < 0) return 2;
        fcntl(fd, F_SETFD, FD_CLOEXEC);
        if (connect(fd, (struct sockaddr *)&address, sizeof(address)) < 0) { close(fd); return 2; }
        NSString *hello = [request[@"token"] stringByAppendingString:@"\n"];
        if (!write_all(fd, hello.UTF8String, strlen(hello.UTF8String))) { close(fd); return 2; }
        char command[6] = {0}; size_t used = 0;
        while (used < 6) {
            struct pollfd ready = { .fd = fd, .events = POLLIN };
            if (poll(&ready, 1, 30000) <= 0) break;
            ssize_t n = read(fd, command + used, 1); if (n != 1) break;
            if (command[used++] == '\n') break;
        }
        if (used != 6 || memcmp(command, "START\n", 6)) { close(fd); return 0; }
        // A stop queued immediately after START must win before any process starts.
        if (stopping(fd)) { event(fd, @"done", nil); close(fd); return 0; }
        NSString *error = [request[@"application"] boolValue]
            ? run_application(fd, request[@"program"], request[@"arguments"], request)
            : run_program(fd, request[@"program"], request[@"arguments"]);
        event(fd, @"done", error);
        close(fd);
        return error ? 1 : 0;
    }
}
int main(int argc, const char **argv) {
    @autoreleasepool {
        if (argc != 2) return 2;
        NSString *directory = [NSString stringWithUTF8String:argv[1]];
        int result = run(directory);
        cleanup(directory);
        return result;
    }
}
