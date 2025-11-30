(function () {
    "use strict";

    const processMsg = (raw) => {
        return raw
            .trim()
            .replaceAll(/\\S[EA]\[\d+\]/g, '')
            .replaceAll(/<I\\\*?item\[(\d+)\]>/g, (s, id) => {
                return window.$dataItems[parseInt(id)]?.name || ''
            })
            .replaceAll(/\\N\[(\d+)\]>/g, (s, id) => {
                return window.$dataCharaters[parseInt(id)]?.name || ''
            })
    }

    let lastMsg = "";
    setInterval(() => {
        if (!window?.$gameMessage?._texts) {
            return;
        }
        const currentMsg = processMsg(window.$gameMessage._texts.join());
        if (lastMsg === currentMsg) {
            return;
        }
        fetch("http://127.0.0.1:17889/submit", {
            method: "POST",
            body: currentMsg,
            headers: { "Content-Type": "text/plain" }
        });
        lastMsg = currentMsg;
    }, 100);
})();