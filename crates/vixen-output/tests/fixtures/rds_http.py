"""Private loopback fixture. Generated CA/cert never enters the system trust store."""
import http.server
import hashlib
import json
import os
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import urllib.request

requests = []
lock = threading.Lock()

class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def log_message(self, *_): pass
    def do_GET(self):
        with lock:
            requests.append({'port': self.server.server_port, 'path': self.path,
                             'authorization': self.headers.get('Authorization'),
                             'cookie': self.headers.get('Cookie'),
                             'host': self.headers.get('Host')})
        parsed = urllib.parse.urlsplit(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        status = 200
        headers = {}
        content = b''
        if parsed.path == '/auth' and not self.headers.get('Authorization'):
            status = 401
            headers['WWW-Authenticate'] = 'Basic realm="rds"'
        elif parsed.path == '/digest':
            authorization = self.headers.get('Authorization', '')
            fields = urllib.request.parse_keqv_list(urllib.request.parse_http_list(authorization.removeprefix('Digest ')))
            md5 = lambda text: hashlib.md5(text.encode()).hexdigest()
            ha1 = md5('user:rds:pass')
            ha2 = md5('GET:' + self.path)
            expected = md5(':'.join([ha1, 'private-test-nonce', fields.get('nc', ''), fields.get('cnonce', ''), 'auth', ha2]))
            if fields.get('response') != expected or fields.get('username') != 'user':
                status = 401
                headers['WWW-Authenticate'] = 'Digest realm="rds", nonce="private-test-nonce", algorithm=MD5, qop="auth"'
        elif parsed.path == '/cookie':
            status = 302
            headers['Set-Cookie'] = 'rds=local; Path=/; HttpOnly'
            headers['Location'] = '/cookie-check'
        elif parsed.path == '/cookie-check' and self.headers.get('Cookie') != 'rds=local':
            status = 403
        elif parsed.path == '/redirect':
            if query.get('auth') and not self.headers.get('Authorization'):
                status = 401
                headers['WWW-Authenticate'] = 'Basic realm="rds"'
            else:
                status = int(query.get('status', ['302'])[0])
                headers['Location'] = query['to'][0]
        elif parsed.path == '/loop':
            status = 302
            headers['Location'] = '/loop'
        elif parsed.path == '/status':
            status = int(query['code'][0])
        elif parsed.path == '/headers':
            headers['X-Large'] = 'x' * (65 * 1024)
        elif parsed.path == '/large':
            content = b'x' * (16 * 1024 * 1024 + 1)
        elif parsed.path == '/slow':
            content = b'x'
        self.send_response(status)
        for name, value in headers.items(): self.send_header(name, value)
        self.send_header('Content-Length', str(len(content)))
        self.send_header('Connection', 'close')
        self.end_headers()
        if parsed.path == '/slow': time.sleep(5)
        try: self.wfile.write(content)
        except (BrokenPipeError, ConnectionResetError, ssl.SSLError): pass

class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True
    def handle_error(self, *_): pass

with tempfile.TemporaryDirectory(prefix='vixen-rds-tls-') as directory:
    def openssl(*args):
        subprocess.run(['openssl', *args], cwd=directory, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=15)
    openssl('req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', 'ca.key',
            '-out', 'ca.pem', '-days', '2', '-subj', '/CN=Vixen private RDS test CA')
    openssl('req', '-newkey', 'rsa:2048', '-nodes', '-keyout', 'server.key',
            '-out', 'server.csr', '-subj', '/CN=localhost')
    with open(os.path.join(directory, 'extensions'), 'w') as out:
        out.write('subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n')
    openssl('x509', '-req', '-in', 'server.csr', '-CA', 'ca.pem', '-CAkey', 'ca.key',
            '-CAcreateserial', '-out', 'server.pem', '-days', '2', '-extfile', 'extensions')
    servers = [Server(('127.0.0.1', 0), Handler) for _ in range(3)]
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(os.path.join(directory, 'server.pem'), os.path.join(directory, 'server.key'))
    servers[2].socket = context.wrap_socket(servers[2].socket, server_side=True)
    for server in servers:
        threading.Thread(target=server.serve_forever, daemon=True).start()
    print(' '.join([str(s.server_port) for s in servers] + [os.path.join(directory, 'ca.pem')]), flush=True)
    # The Rust test requests captured headers over pipes, never through a web API.
    for line in sys.stdin:
        with lock:
            if line.strip() == 'clear': requests.clear(); print('[]', flush=True)
            elif line.strip() == 'requests': print(json.dumps(requests), flush=True)
            elif line.strip() == 'quit': break
    for server in servers: server.shutdown(); server.server_close()
