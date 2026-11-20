
export function genet_begin_frame_scopes(device) {
  const token = {device, pushed: [], closed:false};
  try {
    for (const filter of ['out-of-memory','internal','validation']) {
      device.pushErrorScope(filter); token.pushed.push(filter);
    }
    return token;
  } catch (error) {
    const cleanup = [];
    while (token.pushed.length) {
      token.pushed.pop();
      try { cleanup.push(device.popErrorScope()); } catch (_) {}
    }
    Promise.allSettled(cleanup); throw error;
  }
}
export function genet_seal_frame_scopes(token) {
  if (token.closed) throw new Error('frame scopes already sealed');
  token.closed = true;
  const entries = [];
  while (token.pushed.length) {
    const filter = token.pushed.pop();
    try { entries.push({filter, promise:token.device.popErrorScope()}); }
    catch (error) { entries.push({filter, promise:Promise.reject(error)}); }
  }
  try { entries.push({filter:'queue', promise:token.device.queue.onSubmittedWorkDone()}); }
  catch (error) { entries.push({filter:'queue', promise:Promise.reject(error)}); }
  return Promise.allSettled(entries.map(e => e.promise)).then(results => results.map((r,i) => ({
    filter:entries[i].filter, status:r.status,
    error:r.status === 'fulfilled' ? r.value != null : true,
    message:String(r.status === 'rejected' ? r.reason : r.value?.message ?? '').slice(0,1024)
  })));
}
