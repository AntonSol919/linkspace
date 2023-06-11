<script type="text/javascript">
    // Setup clickable tabs
document.addEventListener('DOMContentLoaded', ()=>
    document.querySelectorAll(".ctabs").forEach((outer,i)=>
        [... outer.children].forEach((el,j) => {
        el.insertAdjacentHTML(
            "beforebegin",
            `<input name="tabs${i}" tabindex="${i}" type="radio" id="tab${i}x${j}" ${j == 0 ? "checked" : ""}>
                 <label for="tab${i}x${j}">${el.className}</label>`
        );
    el.setAttribute("tabindex",i);
        } )
    )
    );

    // Setup highlight js 

document.addEventListener('DOMContentLoaded', (event) => {
        document.querySelectorAll('pre.src').forEach((el) => {
            let lang = el.classList[1].slice(4)
            el.classList.add("language-" + lang)
            hljs.highlightElement(el);
        });
});

</script>

