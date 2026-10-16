export type DrmCapability={keySystem:string;supported:boolean;reason:string};
export async function directCapabilities():Promise<DrmCapability[]>{
 const systems=['com.widevine.alpha','com.microsoft.playready.recommendation','com.microsoft.playready'];
 if(!window.isSecureContext||!navigator.requestMediaKeySystemAccess)return systems.map(keySystem=>({keySystem,supported:false,reason:'Encrypted media API unavailable in this app context'}));
 return Promise.all(systems.map(async keySystem=>{try{await navigator.requestMediaKeySystemAccess(keySystem,[{initDataTypes:['cenc'],audioCapabilities:[{contentType:'audio/mp4; codecs="mp4a.40.2"'}],distinctiveIdentifier:'not-allowed',persistentState:'not-allowed',sessionTypes:['temporary']}]);return {keySystem,supported:true,reason:'Temporary encrypted AAC session supported'};}catch{return {keySystem,supported:false,reason:'Runtime rejected this encrypted-audio configuration'};}}));
}
