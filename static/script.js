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

function initCopyIp(wrap){
  "use strict";
  if(!wrap)return;
  var h1=wrap.querySelector(".ip-display");
  var tooltip=wrap.querySelector(".ip-copied-tooltip");
  if(!h1||!tooltip||!navigator.clipboard)return;
  var timer=0;
  wrap.style.cursor="pointer";
  wrap.setAttribute("role","button");
  wrap.setAttribute("tabindex","0");

  function copyIp(){
    var ip=h1.textContent.trim();
    navigator.clipboard.writeText(ip).then(function(){
      tooltip.hidden=false;
      clearTimeout(timer);
      timer=setTimeout(function(){tooltip.hidden=true;},1500);
    });
  }

  wrap.addEventListener("click",copyIp);
  wrap.addEventListener("keydown",function(e){
    if(e.key==="Enter"||e.key===" "){
      e.preventDefault();
      copyIp();
    }
  });
}

(function(){
  "use strict";
  initCopyIp(document.querySelector(".ip-copy-wrap"));
})();

(function(){
  "use strict";
  var root=document.getElementById("ip-info-root");
  if(!root)return;
  var section=document.getElementById("alt-ip-section");
  if(!section)return;

  var ipv4Domain=root.getAttribute("data-ipv4-domain");
  var copiedText=root.getAttribute("data-copied-text");
  var primaryIp=(root.querySelector(".ip-display")||{}).textContent;
  if(!primaryIp)return;
  primaryIp=primaryIp.trim();

  var primaryIsIPv6=primaryIp.indexOf(":")!==-1;

  function readLabels(){
    var labels={};
    var rows=root.querySelectorAll(".info-table tbody tr");
    for(var i=0;i<rows.length;i++){
      var th=rows[i].querySelector("th");
      var td=rows[i].querySelector("td");
      if(th&&td){
        labels[i]={label:th.textContent.trim(),html:td.innerHTML};
      }
    }
    return labels;
  }

  var existingLabels=readLabels();

  function escapeHtml(s){
    var d=document.createElement("div");
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }

  function boolHtml(val){
    if(val){
      return '<i class="icon-check bool-true" aria-hidden="true"></i>';
    }
    return '<i class="icon-minus bool-false" aria-hidden="true"></i>';
  }

  function labelAt(index){
    return existingLabels[index]?existingLabels[index].label:"";
  }

  function buildBlock(ip,info){
    var html='<div class="ip-copy-wrap" data-copied="'+escapeHtml(copiedText)+'">';
    html+='<h1 class="ip-display">'+escapeHtml(ip)+'</h1>';
    html+='<span class="ip-copied-tooltip" aria-live="polite" hidden>'+escapeHtml(copiedText)+'</span>';
    html+='</div>';

    html+='<table class="info-table"><tbody>';

    var asn=info.asn||{};
    var asnNum=asn.autonomous_system_number;
    var asnOrg=asn.autonomous_system_organization||"";
    var country=info.country||{};
    var countryName=(country.names&&country.names.en)||"";
    var city=info.city||{};
    var cityName=(city.names&&city.names.en)||"";
    var proxy=info.proxy||{};

    if(asnNum){
      var asnStr="AS"+asnNum;
      html+='<tr><th scope="row">'+escapeHtml(labelAt(0))+'</th>';
      html+='<td><a href="https://bgp.tools/as/'+asnNum+'" target="_blank" rel="noopener noreferrer">'+escapeHtml(asnStr)+'</a></td></tr>';
    }
    if(asnOrg){
      html+='<tr><th scope="row">'+escapeHtml(labelAt(1))+'</th>';
      html+='<td>'+escapeHtml(asnOrg)+'</td></tr>';
    }
    if(countryName){
      html+='<tr><th scope="row">'+escapeHtml(labelAt(2))+'</th>';
      html+='<td>'+escapeHtml(countryName)+'</td></tr>';
    }
    if(cityName){
      html+='<tr><th scope="row">'+escapeHtml(labelAt(3))+'</th>';
      html+='<td>'+escapeHtml(cityName)+'</td></tr>';
    }

    var boolStart=Object.keys(existingLabels).length-4;
    html+='<tr><th scope="row">'+escapeHtml(labelAt(boolStart))+'</th><td>'+boolHtml(proxy.is_proxy)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelAt(boolStart+1))+'</th><td>'+boolHtml(proxy.is_vpn)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelAt(boolStart+2))+'</th><td>'+boolHtml(proxy.is_hosting)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelAt(boolStart+3))+'</th><td>'+boolHtml(proxy.is_tor)+'</td></tr>';

    html+='</tbody></table>';
    return html;
  }

  function showAltIp(ip,info){
    section.innerHTML=buildBlock(ip,info);
    section.hidden=false;
    initCopyIp(section.querySelector(".ip-copy-wrap"));
  }

  function fetchJson(ip){
    return fetch("/json?ip="+encodeURIComponent(ip)).then(function(r){
      if(!r.ok)throw new Error(r.status);
      return r.json();
    });
  }

  function detectAltIp(){
    if(primaryIsIPv6&&ipv4Domain){
      var controller=new AbortController();
      var timeout=setTimeout(function(){controller.abort();},5000);
      fetch("https://"+ipv4Domain+"/",{signal:controller.signal})
        .then(function(r){
          clearTimeout(timeout);
          if(!r.ok)throw new Error(r.status);
          return r.text();
        })
        .then(function(text){
          var ip=text.trim();
          if(!ip||ip.indexOf(":")!==-1)return;
          return fetchJson(ip).then(function(info){
            showAltIp(ip,info);
          });
        })
        .catch(function(){});
    }else if(!primaryIsIPv6){
      fetch("/ip")
        .then(function(r){
          if(!r.ok)throw new Error(r.status);
          return r.text();
        })
        .then(function(text){
          var ip=text.trim();
          if(!ip||ip===primaryIp||ip.indexOf(":")===-1)return;
          return fetchJson(ip).then(function(info){
            showAltIp(ip,info);
          });
        })
        .catch(function(){});
    }
  }

  detectAltIp();
})();
