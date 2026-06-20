<script lang="ts">
	import { Channel } from '@tauri-apps/api/core';
	import { app } from '$lib/state.svelte';
	import { settings } from '$lib/settings.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { ChatMessage } from './types';

	// Persona for the assistant. English keeps it provider-neutral; the model is
	// told to answer in the user's language.
	const SYSTEM =
		'You are an AI assistant embedded in AmmaXterm, an SSH terminal. Help the user ' +
		'with shell commands: translate intent into commands, explain commands and error ' +
		'messages, and analyze command output. When you propose a command to run, put ' +
		'exactly that command in a fenced code block so it can be inserted into the ' +
		'terminal. Be concise. Reply in the same language the user writes in.';

	let messages = $state<ChatMessage[]>([]);
	let input = $state('');
	let streaming = $state<string | undefined>(undefined);
	let errorMsg = $state<string | undefined>(undefined);
	let hasKey = $state(false);
	let listEl = $state<HTMLDivElement | undefined>(undefined);

	const needsKey = $derived(settings.s.aiProvider !== 'ollama');
	const providerLabel = $derived(
		settings.s.aiProvider === 'claude'
			? 'Claude'
			: settings.s.aiProvider === 'openai'
				? 'OpenAI'
				: 'Ollama'
	);

	// Re-check the stored key whenever the active provider changes.
	$effect(() => {
		const provider = settings.s.aiProvider;
		if (provider === 'ollama') {
			hasKey = true;
			return;
		}
		app
			.aiHasApiKey(provider)
			.then((v) => (hasKey = v))
			.catch(() => (hasKey = false));
	});

	function errMessage(e: unknown): string {
		return (e as { message?: string })?.message ?? String(e);
	}

	function scrollSoon() {
		queueMicrotask(() => {
			if (listEl) listEl.scrollTop = listEl.scrollHeight;
		});
	}

	async function send(content: string) {
		const text = content.trim();
		if (!text || streaming) return;
		messages.push({ role: 'user', content: text });
		messages.push({ role: 'assistant', content: '' });
		const idx = messages.length - 1;
		const requestId = crypto.randomUUID();
		const ch = new Channel<string>();
		ch.onmessage = (delta) => {
			messages[idx].content += delta;
			scrollSoon();
		};
		streaming = requestId;
		errorMsg = undefined;
		input = '';
		scrollSoon();
		try {
			const history = messages.slice(0, idx).map((m) => ({ role: m.role, content: m.content }));
			await app.aiStream(
				requestId,
				settings.s.aiProvider,
				settings.s.aiModel,
				history,
				SYSTEM,
				ch
			);
		} catch (e) {
			errorMsg = errMessage(e);
			if (!messages[idx].content) messages.splice(idx, 1);
		} finally {
			streaming = undefined;
			scrollSoon();
		}
	}

	function stop() {
		if (streaming) {
			app.aiCancel(streaming);
			streaming = undefined;
		}
	}

	function onSubmit(e: Event) {
		e.preventDefault();
		send(input);
	}

	function composerKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send(input);
		}
	}

	function explainSelection() {
		const sel = app.activeTab?.api?.getSelection() ?? '';
		if (!sel.trim()) {
			errorMsg = i18n.t('ai.noSelection');
			return;
		}
		send(`${i18n.t('ai.explainPrompt')}\n\n\`\`\`\n${sel}\n\`\`\``);
	}

	function analyzeOutput() {
		const txt = app.activeTab?.api?.getRecentText(settings.s.aiContextLines) ?? '';
		if (!txt.trim()) {
			errorMsg = i18n.t('ai.noOutput');
			return;
		}
		send(`${i18n.t('ai.analyzePrompt')}\n\n\`\`\`\n${txt}\n\`\`\``);
	}

	// Pull the command out of the first fenced code block (or use the whole reply),
	// stripped of a trailing newline so it is filled but NOT auto-executed (AI-N2).
	function extractCommand(text: string): string {
		const m = text.match(/```(?:[\w.+-]*)\n([\s\S]*?)```/);
		return (m ? m[1] : text).trim().replace(/\n+$/, '');
	}

	function insertCmd(text: string) {
		const api = app.activeTab?.api;
		if (!api) {
			errorMsg = i18n.t('ai.noSession');
			return;
		}
		api.insert(extractCommand(text));
	}

	function copyText(text: string) {
		void navigator.clipboard.writeText(text);
	}

	function clearChat() {
		messages = [];
		errorMsg = undefined;
	}
</script>

<div class="panel">
	<div class="head">
		<strong>{i18n.t('ai.title')}</strong>
		<span class="model" title={settings.s.aiModel}>{providerLabel} · {settings.s.aiModel}</span>
		<button class="icon" title={i18n.t('ai.clear')} onclick={clearChat} disabled={!messages.length}
			>×</button
		>
	</div>

	{#if !settings.s.aiEnabled}
		<div class="notice">{i18n.t('ai.disabled')}</div>
	{:else if needsKey && !hasKey}
		<div class="notice">{i18n.t('ai.noKey')}</div>
	{:else}
		<div class="messages" bind:this={listEl}>
			{#each messages as m, i (i)}
				<div class="msg {m.role}">
					<div class="who">{m.role === 'user' ? i18n.t('ai.you') : i18n.t('ai.assistant')}</div>
					<div class="bubble">
						{#if !m.content && m.role === 'assistant' && streaming}
							<span class="thinking">{i18n.t('ai.thinking')}</span>
						{:else}{m.content}{/if}
					</div>
					{#if m.role === 'assistant' && m.content}
						<div class="msgactions">
							<button onclick={() => copyText(m.content)}>{i18n.t('ai.copy')}</button>
							<button onclick={() => insertCmd(m.content)}>{i18n.t('ai.insertCommand')}</button>
						</div>
					{/if}
				</div>
			{/each}
			{#if !messages.length}
				<div class="hint">{i18n.t('ai.hint')}</div>
			{/if}
		</div>

		{#if errorMsg}<div class="error">{errorMsg}</div>{/if}

		<div class="quick">
			<button onclick={explainSelection} disabled={!!streaming}>
				{i18n.t('ai.explainSelection')}
			</button>
			<button onclick={analyzeOutput} disabled={!!streaming}>
				{i18n.t('ai.analyzeOutput')}
			</button>
		</div>

		<form class="composer" onsubmit={onSubmit}>
			<textarea
				bind:value={input}
				rows="2"
				placeholder={i18n.t('ai.placeholder')}
				onkeydown={composerKeydown}
			></textarea>
			{#if streaming}
				<button type="button" class="stop" onclick={stop}>{i18n.t('ai.stop')}</button>
			{:else}
				<button type="submit" disabled={!input.trim()}>{i18n.t('ai.send')}</button>
			{/if}
		</form>
		<div class="foot">{i18n.t('ai.privacyNote')}</div>
	{/if}
</div>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--vsc-sidebar-bg);
		color: var(--vsc-sidebar-fg);
		font: 13px var(--vsc-font);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px 6px 12px;
		border-bottom: 1px solid var(--vsc-border);
	}
	.head strong {
		font-size: 12px;
		font-weight: 600;
	}
	.head .model {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 11px;
		color: var(--vsc-muted);
	}
	.icon {
		padding: 2px 8px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--vsc-sidebar-fg);
		font-size: 15px;
		cursor: pointer;
		opacity: 0.6;
	}
	.icon:hover:not(:disabled) {
		opacity: 1;
		background: var(--vsc-button-secondary-hover);
	}
	.icon:disabled {
		opacity: 0.25;
		cursor: default;
	}
	.notice {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		text-align: center;
		color: var(--vsc-muted);
		line-height: 1.5;
	}
	.messages {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 8px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.hint {
		margin: auto;
		padding: 16px;
		text-align: center;
		color: var(--vsc-muted);
		line-height: 1.5;
	}
	.msg {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.who {
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--vsc-muted);
	}
	.bubble {
		padding: 7px 9px;
		border-radius: 6px;
		white-space: pre-wrap;
		word-break: break-word;
		font: 12px/1.5 var(--vsc-font);
	}
	.msg.user .bubble {
		background: var(--vsc-list-active-bg);
		color: var(--vsc-list-fg);
	}
	.msg.assistant .bubble {
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
	}
	.thinking {
		color: var(--vsc-muted);
		font-style: italic;
	}
	.msgactions {
		display: flex;
		gap: 6px;
	}
	.msgactions button {
		padding: 2px 8px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: transparent;
		color: var(--vsc-sidebar-fg);
		font: 11px var(--vsc-font);
		cursor: pointer;
	}
	.msgactions button:hover {
		background: var(--vsc-list-hover-bg);
	}
	.error {
		margin: 0 8px;
		color: var(--vsc-red);
		font-size: 12px;
	}
	.quick {
		display: flex;
		gap: 6px;
		padding: 6px 8px 0;
	}
	.quick button {
		flex: 1;
		padding: 5px 8px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
		font: 11px var(--vsc-font);
		cursor: pointer;
	}
	.quick button:hover:not(:disabled) {
		background: var(--vsc-button-secondary-hover);
	}
	.quick button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.composer {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
	}
	.composer textarea {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
		min-height: 38px;
		padding: 7px 9px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
	}
	.composer textarea:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
		border-color: var(--vsc-focus-border);
	}
	.composer button {
		align-self: flex-end;
		padding: 6px 16px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-bg);
		color: var(--vsc-button-fg);
		font: 12px var(--vsc-font);
		cursor: pointer;
	}
	.composer button:hover:not(:disabled) {
		background: var(--vsc-button-hover);
	}
	.composer button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.composer button.stop {
		background: var(--vsc-red);
	}
	.foot {
		padding: 0 8px 8px;
		font-size: 10px;
		line-height: 1.4;
		color: var(--vsc-muted);
	}
</style>
