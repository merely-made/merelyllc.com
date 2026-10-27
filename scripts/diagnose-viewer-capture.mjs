import {browserArguments} from './browser-launch.mjs';
import {checkQualifiedViewer} from './smoke-viewer.mjs';
import {chromium} from 'playwright';
import {createServer} from 'node:http';
import {readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve('.tmp/viewer-linux-site'),out=path.resolve('.tmp/viewer-linux');
const server=createServer(async(req,res)=>{try{const u=new URL(req.url,'http://localhost');let p=decodeURIComponent(u.pathname);if(p.endsWith('/'))p+='index.html';const f=path.resolve(root,'.'+p);if(!f.startsWith(root+path.sep))throw Error('outside root');const data=await readFile(f);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.wasm':'application/wasm','.svg':'image/svg+xml'})[path.extname(f)]||'application/octet-stream');res.end(data)}catch{res.writeHead(404);res.end()}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({headless:false,args:browserArguments({headless:false,softwareGpu:true})});
try { const result=await checkQualifiedViewer(browser,base,out); await writeFile(path.join(out,'capture-suite.json'),JSON.stringify({browser:browser.version(),fixture:'retained R9 authority with viewer assets from this checkout; see source-binding.json',result},null,2)); console.log(JSON.stringify(result)); } finally { await browser.close(); await new Promise(resolve=>server.close(resolve)); }
