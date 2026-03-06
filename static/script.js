(function(){
  "use strict";
  var form=document.querySelector(".search-form");
  if(!form)return;
  var input=form.querySelector("input[name=ip]");
  var error=document.getElementById("ip-error");
  var msgInvalid=form.getAttribute("data-msg-invalid");
  var msgNonPublic=form.getAttribute("data-msg-non-public");

  function parseIPv4(s){
    var p=s.split(".");
    if(p.length!==4)return null;
    var o=[];
    for(var i=0;i<4;i++){
      if(!/^\d{1,3}$/.test(p[i]))return null;
      var n=parseInt(p[i],10);
      if(n>255||p[i]!==String(n))return null;
      o.push(n);
    }
    return o;
  }

  function isGlobalIPv4(o){
    var a=o[0],b=o[1],c=o[2];
    if(a===0)return false;
    if(a===10)return false;
    if(a===100&&(b&192)===64)return false;
    if(a===127)return false;
    if(a===169&&b===254)return false;
    if(a===172&&(b&240)===16)return false;
    if(a===192&&b===0&&c===0)return false;
    if(a===192&&b===0&&c===2)return false;
    if(a===192&&b===88&&c===99)return false;
    if(a===192&&b===168)return false;
    if(a===198&&(b===18||b===19))return false;
    if(a===198&&b===51&&c===100)return false;
    if(a===203&&b===0&&c===113)return false;
    if(a>=224&&a<=239)return false;
    if(a>=240)return false;
    return true;
  }

  function expandIPv6(s){
    s=s.replace(/^\[|\]$/g,"");
    var halves=s.split("::");
    if(halves.length>2)return null;
    function parseHalf(h){
      if(h==="")return[];
      var parts=h.split(":");
      var last=parts[parts.length-1];
      if(last.indexOf(".")!==-1){
        var v4=parseIPv4(last);
        if(!v4)return null;
        parts.pop();
        parts.push(((v4[0]<<8)|v4[1]).toString(16));
        parts.push(((v4[2]<<8)|v4[3]).toString(16));
      }
      return parts;
    }
    var left=parseHalf(halves[0]);
    if(left===null)return null;
    var right=halves.length===2?parseHalf(halves[1]):[];
    if(right===null)return null;
    if(halves.length===1&&left.length!==8)return null;
    if(halves.length===2){
      var fill=8-left.length-right.length;
      if(fill<0)return null;
      var mid=[];
      for(var i=0;i<fill;i++)mid.push("0");
      left=left.concat(mid,right);
    }
    if(left.length!==8)return null;
    var seg=[];
    for(var j=0;j<8;j++){
      if(!/^[0-9a-fA-F]{1,4}$/.test(left[j]))return null;
      seg.push(parseInt(left[j],16));
    }
    return seg;
  }

  function isGlobalIPv6(seg){
    if(seg[0]===0&&seg[1]===0&&seg[2]===0&&seg[3]===0&&seg[4]===0&&seg[5]===0&&seg[6]===0&&seg[7]===0)return false;
    if(seg[0]===0&&seg[1]===0&&seg[2]===0&&seg[3]===0&&seg[4]===0&&seg[5]===0&&seg[6]===0&&seg[7]===1)return false;
    if(seg[0]===0&&seg[1]===0&&seg[2]===0&&seg[3]===0&&seg[4]===0&&seg[5]===0xFFFF){
      return isGlobalIPv4([seg[6]>>8,seg[6]&255,seg[7]>>8,seg[7]&255]);
    }
    if(seg[0]===0&&seg[1]===0&&seg[2]===0&&seg[3]===0&&seg[4]===0&&seg[5]===0)return false;
    var f=seg[0];
    if((f&0xFFC0)===0xFE80)return false;
    if((f&0xFFC0)===0xFEC0)return false;
    if((f&0xFE00)===0xFC00)return false;
    if((f&0xFF00)===0xFF00)return false;
    if(f===0x0100&&seg[1]===0&&seg[2]===0&&seg[3]===0)return false;
    if(f===0x2001&&seg[1]===0x0DB8)return false;
    if(f===0x2001&&seg[1]===0x0000)return false;
    if(f===0x2002){
      return isGlobalIPv4([seg[1]>>8,seg[1]&255,seg[2]>>8,seg[2]&255]);
    }
    return true;
  }

  function validate(val){
    var s=val.trim();
    if(s==="")return null;
    var v4=parseIPv4(s);
    if(v4)return isGlobalIPv4(v4)?null:msgNonPublic;
    var v6=expandIPv6(s);
    if(v6)return isGlobalIPv6(v6)?null:msgNonPublic;
    return msgInvalid;
  }

  function showError(msg){
    if(msg){
      error.textContent=msg;
      error.hidden=false;
      input.setAttribute("aria-invalid","true");
      input.setAttribute("aria-describedby","ip-error");
    }else{
      error.textContent="";
      error.hidden=true;
      input.removeAttribute("aria-invalid");
      input.removeAttribute("aria-describedby");
    }
  }

  form.addEventListener("submit",function(e){
    var msg=validate(input.value);
    if(msg){
      e.preventDefault();
      showError(msg);
      input.focus();
    }
  });

  input.addEventListener("input",function(){
    if(!error.hidden)showError(validate(input.value));
  });
})();
