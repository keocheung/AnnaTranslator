(function () {
    "use strict";

    const processMsg = (raw) => {
        return raw
            .trim()
            .replaceAll(/\\[A-Z]+\[.+?\]/g, '')
            .replaceAll(/<I\\\*?item\[(\d+)\]>/g, (s, id) => {
                return window.$dataItems[parseInt(id)]?.name || ''
            })
            .replaceAll(/\\N\[(\d+)\]>/g, (s, id) => {
                return window.$dataCharaters[parseInt(id)]?.name || ''
            })
    }

    let lastMsg = null;
    setInterval(() => {
        if (!window?.$gameMessage?._texts || window.$gameMessage._texts.length === 0) {
            return;
        }
        const currentMsg = window.$gameMessage._texts;
        if (lastMsg === currentMsg) {
            return;
        }
        fetch("http://127.0.0.1:17889/submit", {
            method: "POST",
            body: processMsg(currentMsg.join("")),
            headers: { "Content-Type": "text/plain" }
        });
        lastMsg = currentMsg;
    }, 100);
})();