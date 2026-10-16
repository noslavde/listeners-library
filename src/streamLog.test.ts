import {describe,it,expect} from 'vitest';
import {safePlaybackMessage,StreamLog} from './streamLog';
describe('private playback diagnostics',()=>{
 it('redacts URLs, emails, credential fields and opaque payloads without serializing arbitrary objects',()=>{
  const text=safePlaybackMessage('HTTP 403 https://example.test/path?token=secret user@example.test token=credentialvalue '+ 'a'.repeat(100));
  expect(text).toContain('HTTP 403');
  for(const secret of ['secret','user@example','credentialvalue','a'.repeat(100)])expect(text).not.toContain(secret);
  expect(safePlaybackMessage({access_token:'secret'})).not.toContain('secret');
 });
 it('retains the underlying failure when later wrapper errors arrive and bounds retained events',()=>{
  let text='';const log=new StreamLog(next=>{text=next;});
  log.add('Platform license HTTP 403: device registration rejected');log.add('Protected playback error 6007');
  expect(text).toContain('HTTP 403');expect(text).toContain('6007');
  for(let i=0;i<120;i++)log.add('Event '+i);
  expect(text.split('\n')).toHaveLength(100);
  expect(text).not.toContain('Event 19\n');
 });
});
