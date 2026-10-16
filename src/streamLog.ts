export function safePlaybackMessage(value:unknown):string{
 const message=typeof value==='string'?value:value instanceof Error?value.message:'Playback operation failed';
 return message.replace(/https?:\/\/[^\s<>"']+/gi,'[URL omitted]')
  .replace(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi,'[email omitted]')
  .replace(/\b(?:token|authorization|cookie|password|licenseChallenge|license_response)\s*[:=]\s*\S+/gi,'[private field omitted]')
  .replace(/[A-Za-z0-9+/_=-]{48,}/g,'[opaque value omitted]').slice(0,400);
}
export class StreamLog{
 private started=performance.now();private lines:string[]=[];
 constructor(private changed:(text:string)=>void){}
 add(message:unknown){this.lines.push(`+${((performance.now()-this.started)/1000).toFixed(2)}s ${safePlaybackMessage(message)}`);this.lines=this.lines.slice(-100);this.changed(this.lines.join('\n'));}
}
