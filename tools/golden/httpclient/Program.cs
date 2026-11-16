using System.Net;
using System.Text.Json;
int pa=int.Parse(args[0]),pb=int.Parse(args[1]),pt=int.Parse(args[2]);
var result=new List<object>();
var cases = new List<(string name,string url,bool cache,string method,bool credentials)> {
 ("same",$"http://127.0.0.1:{pa}/auth/same",false,"GET",true), ("crossport",$"http://127.0.0.1:{pa}/auth/crossport",false,"GET",true), ("crosshost",$"http://127.0.0.1:{pa}/auth/crosshost",false,"GET",true), ("plain-redirect-auth",$"http://127.0.0.1:{pa}/plain/same",false,"GET",true), ("credential-cache-control",$"http://127.0.0.1:{pa}/auth/same",true,"GET",true), ("redirect-limit",$"http://127.0.0.1:{pa}/redirect/0",false,"GET",true), ("tls-downgrade",$"https://127.0.0.1:{pt}/tls/down",false,"GET",true), ("tls-upgrade",$"http://127.0.0.1:{pa}/http/up",false,"GET",true),
 ("userinfo-only",$"http://uri-user:uri-password@127.0.0.1:{pa}/finish/basic",false,"GET",false), ("userinfo-and-credentials",$"http://uri-user:uri-password@127.0.0.1:{pa}/finish/basic",false,"GET",true)
};
cases.Add(("cookie-redirect",$"http://127.0.0.1:{pa}/cookie/start",false,"GET",false));
cases.Add(("cookie-isolated",$"http://127.0.0.1:{pa}/cookie/need",false,"GET",false));
foreach(int code in new[]{300,301,302,303,304,305,306,307,308}) foreach(var method in new[]{"GET","POST"}) cases.Add(($"{method}-{code}",$"http://127.0.0.1:{pa}/status/{code}",false,method,false));
foreach(var scheme in new[]{"file","ftp"})cases.Add(($"redirect-{scheme}",$"http://127.0.0.1:{pa}/scheme/{scheme}",false,"GET",true));
foreach(var entry in cases) {
 using var handler=new HttpClientHandler();
 if(entry.credentials)handler.Credentials=new NetworkCredential("local-user","local-password");
 if(entry.cache) {var cache=new CredentialCache();cache.Add(new Uri($"http://127.0.0.1:{pa}/"),"Basic",new NetworkCredential("local-user","local-password"));handler.Credentials=cache;}
 // Test-only trust of the ephemeral loopback server; no system keychain change.
 handler.ServerCertificateCustomValidationCallback=(m,c,ch,e)=>true;
 using var http=new HttpClient(handler){Timeout=TimeSpan.FromSeconds(10)};
 http.DefaultRequestHeaders.Add("X-Test-Case",entry.name);
 try { using var request=new HttpRequestMessage(new HttpMethod(entry.method),entry.url);if(entry.method=="POST")request.Content=new StreamContent(new MemoryStream(System.Text.Encoding.ASCII.GetBytes("original-stream-body")));using var response=await http.SendAsync(request); result.Add(new {name=entry.name,status=(int)response.StatusCode,finalUri=response.RequestMessage!.RequestUri!.ToString(),maxRedirects=handler.MaxAutomaticRedirections}); }
 catch(Exception e) {result.Add(new {name=entry.name,error=e.GetType().Name+": "+e.Message});}
}
using(var http=new HttpClient(new HttpClientHandler())) {
 http.DefaultRequestHeaders.Add("X-Test-Case","cookie-persist");
 using var first=await http.GetAsync($"http://127.0.0.1:{pa}/cookie/start");
 using var second=await http.GetAsync($"http://127.0.0.1:{pa}/cookie/need");
 result.Add(new {name="cookie-persist",status=(int)second.StatusCode});
}
Console.WriteLine(JsonSerializer.Serialize(new {runtime=Environment.Version.ToString(),os=Environment.OSVersion.ToString(),result},new JsonSerializerOptions{WriteIndented=true}));
