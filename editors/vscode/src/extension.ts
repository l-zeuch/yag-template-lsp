import { resolve } from 'path';
import { commands, ExtensionContext, window, workspace, WorkspaceConfiguration } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient | undefined = undefined;

// Keep in sync with `YAG_LSP_SECTION_NAME` in yag-template-lsp crate.
const configSection = 'yagTemplate';

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export async function activate(context: ExtensionContext) {
	const config = workspace.getConfiguration(configSection);

	context.subscriptions.push(commands.registerCommand('yag-template-lsp.restartServer', restartServer));
	try {
		await startClient(config);
	} catch (error) {
		void window.showErrorMessage(`Failed to activate yag-template-lsp: ${error}`);
		throw error;
	}
}

export function deactivate() {
	return client?.stop();
}

function restartServer() {
	void client?.restart();
}

async function startClient(config: WorkspaceConfiguration) {
	const extraEnv = config.get<Record<string, string> | null>('server.extraEnv') ?? {};
	const run = {
		command: getLanguageServerBinary(config),
		options: { env: { ...process.env, ...extraEnv, RUST_BACKTRACE: '1' } },
	};

	const serverOptions: ServerOptions = {
		run,
		debug: run,
	};

	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ scheme: 'file', language: 'yag' }],
		initializationOptions: config,
		synchronize: { configurationSection: configSection },
	};

	client = new LanguageClient(configSection, 'YAGPDB Template Language Server', serverOptions, clientOptions);
	return client.start();
}

function getLanguageServerBinary(config: WorkspaceConfiguration) {
	const localServerPath = config.get<string | null>('server.path');
	return localServerPath || bundledLanguageServer();
}

function bundledLanguageServer() {
	if (process.platform === 'win32') return resolve(__dirname, 'yag-template-lsp.exe');
	return resolve(__dirname, 'yag-template-lsp');
}
