# Analysis of UI and MCP Issues

## 1. Gamification Elements "Floating" out of nowhere
The user noted that Ludus/gamification elements look floating and lack context. In `CompanionHUD.tsx` and the newly created `UnifiedDashboard.tsx`, elements might lack proper contextual hierarchy or backgrounds.
**Plan**: I will map Ludus XP markers explicitly to Agent Operation nodes or tightly couple them into a "HUD bar" rather than a floating window.

## 2. Scrollbars Missing
The user mentioned "there's no scroll ballers".
In `UnifiedDashboard.tsx`, I used `overflow-y-auto` but the styling in `index.css` or VS Code might hide scrollbars natively, or the `h-full` containers are not constrained correctly, preventing scrollbars from appearing at all. I will replace hardcoded pixel heights with `flex-1 min-h-0 overflow-y-auto` patterns properly.

## 3. Element Alignment
The user reported "better alignment of elements that might not be arranged pleasingly". I will review Flex grids across `EngineeringDiagnostics` and `UnifiedDashboard` for consistent `gap` and `items-center` usage, ensuring text sizes align to baseline.

## 4. MCP Error -32000
"Look at what gender it's that in the MCP... I may have to run something from the command line. Try and serve it if you need to."
The user suspects the error has a deeper source ("Try and serve it if you need to"). The `vox mcp` server over stdio runs correctly in my background test (`target/debug/vox.exe mcp`). But in the VS Code host, `ConfigManager.mcpServerPath` might be resolving to a global `vox` executable instead of the updated `target/debug/vox.exe`.
I will explicitly insert debug logs into `VoxMcpClient.ts` to print the EXACT path spawned and I will force `const cmd = vscode.workspace.getConfiguration('vox').get('mcp.serverPath') || 'vox';` resolution logic to be more resilient, perhaps logging exact arguments. I will also check if `StdioClientTransport` is discarding stderr, hiding the true error.

## 5. Tool Discovery
"how we can better discover tools"
The user wants better tool discovery. Maybe I should list the loaded capability schemas directly in the EngineeringDiagnostics page.

I will formulate a plan.
