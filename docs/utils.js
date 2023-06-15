// Setup clickable tabs
document.addEventListener('DOMContentLoaded', () =>
    document.querySelectorAll(".ctabs").forEach((outer, i) => [...outer.children].forEach((el, j) => {

        let forName = `tabs${i}`;
        let checkedEl = window.localStorage.getItem(forName) || 0;
        let checkbox = `
<input
name="${forName}"
tabindex="${i}"
type="radio"
id="tab${i}x${j}"
${j == checkedEl ? "checked" : ""}
/>
<label for="tab${i}x${j}" class="checkbox-${el.classList[0]}" >${el.className}</label>`;

        el.insertAdjacentHTML(
            "beforebegin", checkbox
        );
        outer.querySelector(`#tab${i}x${j}`).addEventListener("change", () => {
            window.localStorage.setItem(forName, j);
        })
        el.setAttribute("tabindex", i);
    }))
);

document.addEventListener('DOMContentLoaded', (_) => {
    document.querySelectorAll('pre.src').forEach((el) => {
        let lang = el.classList[1].slice(4);
        el.classList.add("language-" + lang);
        hljs && hljs.highlightElement(el);
    });
});