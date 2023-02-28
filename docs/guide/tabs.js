document.addEventListener('DOMContentLoaded', ()=>
    document.querySelectorAll(".ctabs").forEach((outer,i)=>
        [... outer.children].forEach((el,j) => {
            el.insertAdjacentHTML(
                "beforebegin",
                `<input name="tabs${i}" tabindex="${i}" type="radio" id="tab${i}x${j}" ${ j == 0 ? "checked" : "" }>
                 <label for="tab${i}x${j}">${el.className}</label>`
            );
            el.setAttribute("tabindex",i);
        } )
    )
);
