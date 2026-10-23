// Original DynCursor artwork. Export conventional 32-bit DIB CURs with alpha and AND masks.
const sharp = require('sharp');
const fs = require('fs');
const path = require('path');
const root = path.resolve(__dirname,'..');
const artwork = {
  arrow: {hotspot:[5,3],body:'<path d="M5 3v45l12-12 10 23 9-4-10-22h18z"/>'},
  beam: {hotspot:[31,31],body:'<path d="M21 5h22v6h-7v42h7v6H21v-6h7V11h-7z"/>'},
  link: {hotspot:[25,4],body:'<path d="M21 33V9c0-7 10-7 10 0v18c0-6 10-6 10 0v3c0-5 9-5 9 1v3c0-4 9-4 9 2v8c0 9-7 15-16 15H32c-6 0-10-4-13-9L8 34c-4-6 3-11 7-6l6 7z"/>'}
};
(async()=>{
  for(const [name,{hotspot,body}] of Object.entries(artwork)){
    const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><g fill="#126344" stroke="#ffffff" stroke-width="2" stroke-linejoin="round">${body}</g></svg>`;
    const size=32;
    const {data}=await sharp(Buffer.from(svg)).resize(size,size).ensureAlpha().raw().toBuffer({resolveWithObject:true});
    const stride=Math.ceil(size/32)*4,pixels=Buffer.alloc(size*size*4),mask=Buffer.alloc(stride*size);
    for(let y=0;y<size;y++)for(let x=0;x<size;x++){
      const from=(y*size+x)*4,to=((size-1-y)*size+x)*4;
      pixels[to]=data[from+2];pixels[to+1]=data[from+1];pixels[to+2]=data[from];pixels[to+3]=data[from+3];
      if(data[from+3]===0)mask[(size-1-y)*stride+Math.floor(x/8)]|=0x80>>(x%8);
    }
    const dib=Buffer.alloc(40);dib.writeUInt32LE(40,0);dib.writeInt32LE(size,4);dib.writeInt32LE(size*2,8);dib.writeUInt16LE(1,12);dib.writeUInt16LE(32,14);dib.writeUInt32LE(pixels.length+mask.length,20);
    const header=Buffer.alloc(22);header.writeUInt16LE(2,2);header.writeUInt16LE(1,4);header[6]=size;header[7]=size;header.writeUInt16LE(Math.round(hotspot[0]/2),10);header.writeUInt16LE(Math.round(hotspot[1]/2),12);header.writeUInt32LE(dib.length+pixels.length+mask.length,14);header.writeUInt32LE(22,18);
    fs.writeFileSync(path.join(root,`resources/cur/custom-${name}.cur`),Buffer.concat([header,dib,pixels,mask]));
    fs.mkdirSync(path.join(root,'resources/cursor-source'),{recursive:true});
    fs.writeFileSync(path.join(root,`resources/cursor-source/${name}.svg`),svg+'\n');
  }
})();
