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

function scaleIpv6(el){
  if(!el)return;
  var ip=el.textContent.trim();
  if(ip.indexOf(":")===-1)return;
  var len=ip.length;
  // IPv6 addresses range from ~5 chars (e.g. "::1") to 39 chars (full form).
  // Scale font down for longer addresses. Short IPv6 (<= 16 chars) keeps default.
  // At 39 chars (max), scale to ~60% of default size.
  if(len<=16)return;
  var scale=1-((len-16)/(39-16))*0.4;
  if(scale<0.6)scale=0.6;
  var maxPx=Math.round(42*scale);
  var vw=+(6*scale).toFixed(2);
  var minPx=Math.round(28*scale);
  el.style.fontSize="clamp("+minPx+"px, "+vw+"vw, "+maxPx+"px)";
}

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
  var wrap=document.querySelector(".ip-copy-wrap");
  initCopyIp(wrap);
  if(wrap)scaleIpv6(wrap.querySelector(".ip-display"));
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

  var langTag=document.documentElement.getAttribute("lang")||"en";
  var mmdbKeyMap={"en":"en","de":"de","es":"es","fr":"fr","ja":"ja","ru":"ru","pt":"pt-BR","zh-Hans":"zh-CN","zh-Hant":"zh-CN"};
  var mmdbKey=mmdbKeyMap[langTag]||"en";

  function localizedName(names){
    if(!names)return "";
    return names[mmdbKey]||names.en||"";
  }

  function readLabels(){
    var labels={};
    var rows=root.querySelectorAll(".info-table tbody tr");
    for(var i=0;i<rows.length;i++){
      var key=rows[i].getAttribute("data-label");
      var th=rows[i].querySelector("th");
      if(key&&th){
        labels[key]=th.textContent.trim();
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

  function labelFor(key){
    return existingLabels[key]||"";
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
    var countryName=localizedName(country.names);
    var city=info.city||{};
    var cityName=localizedName(city.names);
    var proxy=info.proxy||{};

    if(asnNum){
      var asnStr="AS"+asnNum;
      html+='<tr><th scope="row">'+escapeHtml(labelFor("asn"))+'</th>';
      html+='<td><a href="https://bgp.tools/as/'+asnNum+'" target="_blank" rel="noopener noreferrer">'+escapeHtml(asnStr)+'</a></td></tr>';
    }
    if(asnOrg){
      html+='<tr><th scope="row">'+escapeHtml(labelFor("org"))+'</th>';
      html+='<td>'+escapeHtml(asnOrg)+'</td></tr>';
    }
    if(countryName){
      html+='<tr><th scope="row">'+escapeHtml(labelFor("country"))+'</th>';
      html+='<td>'+escapeHtml(countryName)+'</td></tr>';
    }
    if(cityName){
      html+='<tr><th scope="row">'+escapeHtml(labelFor("city"))+'</th>';
      html+='<td>'+escapeHtml(cityName)+'</td></tr>';
    }

    html+='<tr><th scope="row">'+escapeHtml(labelFor("proxy"))+'</th><td>'+boolHtml(proxy.is_proxy)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelFor("vpn"))+'</th><td>'+boolHtml(proxy.is_vpn)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelFor("hosting"))+'</th><td>'+boolHtml(proxy.is_hosting)+'</td></tr>';
    html+='<tr><th scope="row">'+escapeHtml(labelFor("tor"))+'</th><td>'+boolHtml(proxy.is_tor)+'</td></tr>';

    html+='</tbody></table>';
    return html;
  }

  function buildSkeleton(){
    var html='<div class="skeleton-block">';
    html+='<div class="skeleton-bone skeleton-ip"></div>';
    html+='</div>';
    html+='<div class="skeleton-table">';
    for(var i=0;i<8;i++){
      html+='<div class="skeleton-row">';
      html+='<div class="skeleton-row-label"><div class="skeleton-bone"></div></div>';
      html+='<div class="skeleton-row-value"><div class="skeleton-bone"></div></div>';
      html+='</div>';
    }
    html+='</div>';
    return html;
  }

  function showSkeleton(){
    section.innerHTML=buildSkeleton();
    section.hidden=false;
    section.classList.add("alt-visible");
  }

  function showAltIp(ip,info){
    section.classList.remove("alt-visible");
    setTimeout(function(){
      section.innerHTML=buildBlock(ip,info);
      initCopyIp(section.querySelector(".ip-copy-wrap"));
      scaleIpv6(section.querySelector(".ip-display"));
      section.hidden=false;
      // Force reflow before adding the visible class for transition
      void section.offsetHeight;
      section.classList.add("alt-visible");
    },primaryIsIPv6?300:0);
  }

  function fetchJson(ip){
    return fetch("/json?ip="+encodeURIComponent(ip)).then(function(r){
      if(!r.ok)throw new Error(r.status);
      return r.json();
    });
  }

  function hideSkeleton(){
    section.classList.remove("alt-visible");
    setTimeout(function(){
      section.innerHTML="";
      section.hidden=true;
    },300);
  }

  function detectAltIp(){
    if(primaryIsIPv6&&ipv4Domain){
      showSkeleton();
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
          if(!ip||ip.indexOf(":")!==-1){hideSkeleton();return;}
          return fetchJson(ip).then(function(info){
            showAltIp(ip,info);
          });
        })
        .catch(function(){hideSkeleton();});
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

  if(!root.hasAttribute("data-is-query")){
    detectAltIp();
  }
})();
