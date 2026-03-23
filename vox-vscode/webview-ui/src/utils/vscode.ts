/**
 * Utility to interact with the VS Code API from the webview.
 */
declare const acquireVsCodeApi: () => {
    postMessage: (message: any) => void;
    setState: (state: any) => void;
    getState: () => any;
};

let vscodeApi: any = null;

export function getVsCodeApi() {
    if (!vscodeApi) {
        if (typeof acquireVsCodeApi !== "undefined") {
            vscodeApi = acquireVsCodeApi();
        } else {
            // Fallback for browser testing
            vscodeApi = {
                postMessage: (msg: any) => console.log("VSCode PostMessage:", msg),
                setState: (s: any) => console.log("VSCode SetState:", s),
                getState: () => null,
            };
        }
    }
    return vscodeApi;
}
