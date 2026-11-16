import http.server,json,pathlib,ssl,subprocess,threading,sys,tempfile,argparse
repo=pathlib.Path(__file__).resolve().parents[3]
parser=argparse.ArgumentParser()
parser.add_argument('--out',type=pathlib.Path,default=repo/'target/golden/httpclient-reference')
args=parser.parse_args()
root=args.out.resolve()
root.mkdir(parents=True,exist_ok=True)
private=tempfile.TemporaryDirectory(prefix='vixen-httpclient-oracle-')
certdir=pathlib.Path(private.name)
traces=[]
servers={}
class Handler(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  auth=self.headers.get('Authorization') is not None
  body=self.rfile.read(int(self.headers.get('Content-Length','0')))
  traces.append(dict(name=self.headers.get('X-Test-Case'),server=self.server.label,path=self.path,auth=auth,authorization=self.headers.get('Authorization'),method=self.command,body=body.decode('ascii'),cookie=self.headers.get('Cookie')))
  code=200; extra={}
  if (self.path.startswith('/auth/') or self.path=='/finish/basic') and not auth:
   code=401;extra={'WWW-Authenticate':'Basic realm="local"'}
  elif self.path.startswith('/auth/'):
   code=302;extra={'Location':{'same':'/finish/basic','crossport':f'http://127.0.0.1:{servers["b"].server_port}/finish/basic','crosshost':f'http://localhost:{servers["a"].server_port}/finish/basic'}[self.path[6:]]}
  elif self.path=='/cookie/start':code=302;extra={'Location':'/cookie/need','Set-Cookie':'fixture=local; Path=/'}
  elif self.path=='/cookie/need':code=200 if self.headers.get('Cookie')=='fixture=local' else 403
  elif self.path=='/plain/same':code=302;extra={'Location':'/finish/basic'}
  elif self.path.startswith('/redirect/'):code=302;extra={'Location':f'/redirect/{int(self.path[10:])+1}'}
  elif self.path=='/tls/down':code=302;extra={'Location':f'http://127.0.0.1:{servers["a"].server_port}/finish/plain'}
  elif self.path.startswith('/status/'):
   code=int(self.path[8:]);extra={'Location':'/finish/plain'}
  elif self.path.startswith('/scheme/'):
   code=302;extra={'Location':{'file':'file:///private/tmp/unused-httpclient-probe','ftp':f'ftp://127.0.0.1:{servers["b"].server_port}/unused'}[self.path[8:]]}
  elif self.path=='/http/up':code=302;extra={'Location':f'https://127.0.0.1:{servers["tls"].server_port}/finish/plain'}
  self.send_response(code)
  for k,v in extra.items():self.send_header(k,v)
  self.send_header('Content-Length','0');self.send_header('Connection','close');self.end_headers()
 do_POST=do_GET
 def log_message(self,*args):pass
subprocess.run(['/usr/bin/openssl','req','-x509','-newkey','rsa:2048','-nodes','-keyout',str(certdir/'key.pem'),'-out',str(certdir/'cert.pem'),'-subj','/CN=localhost','-days','1'],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=15)
try:
 for label in ['a','b','tls']:
  server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);server.label=label
  if label=='tls':
   context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);context.load_cert_chain(str(certdir/'cert.pem'),str(certdir/'key.pem'));server.socket=context.wrap_socket(server.socket,server_side=True)
  servers[label]=server;threading.Thread(target=server.serve_forever,daemon=True).start()
 run=subprocess.run([str(repo/'target/golden/dotnet-root/dotnet'),str(repo/'target/golden/httpclient/bin/HttpClientReference.dll'),*[str(servers[k].server_port) for k in ['a','b','tls']]],capture_output=True,text=True,timeout=120)
 if run.returncode:raise RuntimeError(run.stderr)
 data=json.loads(run.stdout)
 for r in data['result']:r['trace']=[t for t in traces if t['name']==r['name']]
 (root/'results.json').write_text(json.dumps(data,indent=2)+'\n')
 by_name={r['name']:r for r in data['result']}
 for name in ['same','crossport','crosshost']:
  r=by_name[name];assert r['status']==401 and [t['auth'] for t in r['trace']]==[False,True,False],r
 assert by_name['plain-redirect-auth']['status']==401
 assert by_name['credential-cache-control']['status']==200
 assert by_name['redirect-limit']['status']==302 and len(by_name['redirect-limit']['trace'])==51
 assert by_name['tls-downgrade']['status']==302 and len(by_name['tls-downgrade']['trace'])==1
 assert by_name['tls-upgrade']['status']==200 and len(by_name['tls-upgrade']['trace'])==2
 assert by_name['userinfo-only']['status']==401 and len(by_name['userinfo-only']['trace'])==1
 assert by_name['userinfo-and-credentials']['status']==200
 for code in [300,301,302,303,304,305,306,307,308]:
  for method in ['GET','POST']:
   r=by_name[f'{method}-{code}'];follow=code not in [304,305,306]
   assert r['status']==(200 if follow else code),r
   expected='GET' if follow and code in [300,301,302,303] else method
   assert r['trace'][-1]['method']==expected,r
   assert r['trace'][-1]['body']==('original-stream-body' if expected=='POST' else ''),r
 assert by_name['cookie-redirect']['status']==200 and [t['cookie'] for t in by_name['cookie-redirect']['trace']]==[None,'fixture=local']
 assert by_name['cookie-isolated']['status']==403 and by_name['cookie-isolated']['trace'][0]['cookie'] is None
 assert by_name['cookie-persist']['status']==200 and [t['cookie'] for t in by_name['cookie-persist']['trace']]==[None,'fixture=local','fixture=local']
 assert 'UriFormatException' in by_name['redirect-file']['error']
 assert by_name['redirect-ftp']['status']==200
 print(f"PASS {len(data['result'])} HttpClient cases on .NET {data['runtime']}; {root/'results.json'}")
finally:
 for server in servers.values():server.shutdown();server.server_close()
 private.cleanup()
