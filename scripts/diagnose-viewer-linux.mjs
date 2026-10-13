import {chromium} from 'playwright';
import {createServer} from 'node:http';
import {readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve('.tmp/viewer-linux-site'),out=path.resolve('.tmp/viewer-linux');
const server=createServer(async(req,res)=>{try{const u=new URL(req.url,'http://localhost');let p=decodeURIComponent(u.pathname);if(p.endsWith('/'))p+='index.html';const f=path.resolve(root,'.'+p);if(!f.startsWith(root+path.sep))throw Error('outside root');const data=await readFile(f);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.json':'application/json','.wasm':'application/wasm','.svg':'image/svg+xml'})[path.extname(f)]||'application/octet-stream');res.end(data)}catch{res.writeHead(404);res.end()}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base=`http://127.0.0.1:${server.address().port}`;
const modes={
 swangle:['--enable-unsafe-webgpu','--use-gl=angle','--use-angle=swiftshader','--disable-gpu-watchdog'],
 shared_vulkan:['--enable-unsafe-webgpu','--use-gl=angle','--use-angle=swiftshader','--use-vulkan=swiftshader','--enable-features=Vulkan,VulkanFromANGLE,DefaultANGLEVulkan','--disable-vulkan-surface','--disable-gpu-watchdog'],
 vulkan:['--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--use-vulkan=swiftshader','--use-webgpu-adapter=swiftshader','--disable-vulkan-surface','--disable-gpu-watchdog']
};
try{for(const [mode,args] of Object.entries(modes)){
 const browser=await chromium.launch({headless:false,args:[...args,'--enable-logging=stderr']});const page=await browser.newPage({viewport:{width:1440,height:1000}});const result={mode,args,browser:browser.version(),console:[],pageErrors:[]};
 page.on('console',m=>result.console.push({type:m.type(),text:m.text()}));page.on('pageerror',e=>result.pageErrors.push(String(e)));
 try{
  await page.addInitScript(()=>{window.__gpu={devices:[],errors:[],lost:[]};window.__gpuDevices=[];const original=GPUAdapter.prototype.requestDevice;GPUAdapter.prototype.requestDevice=async function(...args){window.__gpu.devices.push({vendor:this.info.vendor,architecture:this.info.architecture,device:this.info.device,description:this.info.description});const d=await original.apply(this,args);window.__gpuDevices.push(d);d.addEventListener('uncapturederror',e=>window.__gpu.errors.push(e.error.message));d.lost.then(e=>window.__gpu.lost.push({reason:e.reason,message:e.message}));return d;};});
  const cdp=await browser.newBrowserCDPSession();result.system=(await cdp.send('SystemInfo.getInfo')).gpu;await cdp.detach();
  await page.goto(base+'/repos/',{waitUntil:'networkidle'});await page.bringToFront();await page.getByRole('button',{name:'Open the live graph',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('[data-graph-sandbox]').dataset.sandboxState==='ready',null,{timeout:60000,polling:100});
  const viewer=page.locator('graphshell-tree');await viewer.scrollIntoViewIfNeeded();
  await viewer.getByRole('button',{name:'Choose projects',exact:true}).press('Enter');
  await viewer.getByRole('button',{name:/^Select /}).first().waitFor({state:'attached',timeout:10000});
  result.choices=await viewer.getByRole('button',{name:/^Select /}).count();
  result.queue=await page.evaluate(()=>Promise.race([Promise.all(window.__gpuDevices.map(d=>d.queue.onSubmittedWorkDone())).then(()=> 'complete',e=>String(e)),new Promise(resolve=>setTimeout(()=>resolve('timeout'),30000))]));
  await page.waitForTimeout(1000);result.pass=true;
 }catch(error){result.failure=String(error);result.pass=false;}
 finally{
  result.state=await page.evaluate(()=>({gpu:window.__gpu,hidden:document.hidden,title:document.title,ready:document.querySelector('graphshell-tree')?.dataset.ready,active:document.activeElement?.outerHTML,mirror:document.querySelector('[data-cambium-mirror]')?.textContent})).catch(e=>String(e));
  await page.screenshot({path:path.join(out,mode+'.png')}).catch(()=>{});await page.locator('graphshell-tree').screenshot({path:path.join(out,mode+'-viewer.png')}).catch(()=>{});
  await writeFile(path.join(out,mode+'.json'),JSON.stringify(result,null,2));console.log(mode,JSON.stringify({pass:result.pass,failure:result.failure,gpu:result.state?.gpu,pageErrors:result.pageErrors}));await browser.close();
 }
}}finally{await new Promise(resolve=>server.close(resolve));}
