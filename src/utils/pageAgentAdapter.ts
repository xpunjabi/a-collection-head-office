/**
 * v0.23.0 — Page-Agent Adapter (Phase 1: Foundation)
 * ===================================================
 *
 * Lazy singleton that constructs a `PageAgentCore` instance wired to use
 * our `createTauriFetch` bridge. PageAgentCore is the headless variant —
 * no built-in Panel UI. We drive it from our existing AiWorkspace UI.
 *
 * DESIGN:
 *   - Lazy: Page-Agent is only loaded when the user first toggles Agent
 *     Mode in AiWorkspace. Bundle cost paid on demand, not at app start.
 *   - Singleton: One agent instance per app session. Re-used across tasks.
 *     Page-Agent maintains its own conversation history per task.
 *   - Disposable: When Agent Mode is toggled off, agent.dispose() releases
 *     DOM listeners. Next toggle re-creates the singleton.
 *
 * ARCHITECTURE (matches the blueprint):
 *
 *   React UI (AiWorkspace) ──┐
 *                            │ getAgent().execute(task)
 *                            ▼
 *                    ┌──────────────────────┐
 *                    │  pageAgentAdapter.ts │
 *                    │  (this file)         │
 *                    └──────────┬───────────┘
 *                               │ customFetch intercepts HTTP
 *                               ▼
 *                    ┌──────────────────────┐
 *                    │  pageAgentBridge.ts  │
 *                    │  (createTauriFetch)  │
 *                    └──────────┬───────────┘
 *                               │ invoke('page_agent_invoke', ...)
 *                               ▼
 *                    ┌──────────────────────┐
 *                    │  Rust: ai/mod.rs     │
 *                    │  call_ai_provider()  │  ← UNCHANGED
 *                    └──────────┬───────────┘
 *                               │
 *                               ▼
 *                    Gemini / OpenAI / Claude
 *                    (per existing ai_provider setting)
 */

import { invoke } from '@tauri-apps/api/core'
import { PageAgentCore } from '@page-agent/core'
import { PageController } from '@page-agent/page-controller'
import { createTauriFetch } from './pageAgentBridge'

import { useAppStore } from '../stores/store'

/**
 * Dummy LLM config — required by PageAgentCore's constructor but never
 * actually used for HTTP because our `customFetch` intercepts every call
 * before any network request is made. The `baseURL` and `apiKey` values
 * here are intentionally fake; the real provider/key/model live in the
 * Rust `settings` table and are read by `page_agent_invoke`.
 *
 * Note: parseLLMConfig in @page-agent/llms throws if baseURL or model is
 * empty, so we must provide non-empty strings.
 */
const DUMMY_LLM_CONFIG = {
  baseURL: 'https://tauri-bridge.local/v1',
  model: 'tauri-bridge',
  apiKey: 'not-used-rust-holds-the-real-key',
  maxRetries: 1, // we already get retries from call_ai_provider's 45s timeout
}

let agentInstance: PageAgentCore | null = null

/**
 * Lazy-initialise and return the singleton PageAgentCore instance.
 *
 * On first call:
 *   1. Construct a PageController (DOM reader, no mask overlay)
 *   2. Construct PageAgentCore with:
 *      - dummy LLM config (see above)
 *      - our customFetch bridge (the actual LLM routing)
 *      - per-page instructions callback that reads business context
 *        from Rust via the `page_agent_get_context` command
 *   3. Store as singleton
 *
 * Subsequent calls return the cached instance.
 *
 * Call `disposeAgent()` to release DOM listeners and clear the singleton.
 */
export async function getAgent(): Promise<PageAgentCore> {
  if (agentInstance && !agentInstance.disposed) {
    return agentInstance
  }

  // PageController — no visual mask overlay for Phase 1. Ali bhai's UI
  // stays clean; agent operates invisibly. (enableMask: false is also the
  // PageController default, but explicit is better than implicit.)
  const pageController = new PageController({ enableMask: false })

  agentInstance = new PageAgentCore({
    ...DUMMY_LLM_CONFIG,
    pageController,
    customFetch: createTauriFetch(),

    // Reasonable step budget for Phase 1. Phase 2+ can tune per task.
    maxSteps: 15,

    // English-only UI labels (Hinglish spoken language is handled by the
    // model via system prompt, not by Page-Agent's UI strings).
    language: 'en-US',

    // Per-page instructions: fetch minimal app context from Rust so the
    // agent's system prompt knows the current tab + counts. This is the
    // `getPageInstructions(url)` hook from PageAgentCore config.
    instructions: {
      system:
        'You are operating the A Collection Head Office desktop app — a clothing retail management system for Ali bhai in Narowal, Pakistan. ' +
        'The user is the business owner, age 49, not highly tech-savvy. ' +
        'Be concise. Prefer reading what is already on screen over clicking around. ' +
        'When you have enough information to answer, call the "done" tool immediately — do not explore unnecessarily.',
      getPageInstructions: (_url: string) => {
        // Synchronous callback — we can't `await invoke()` here. Instead,
        // we return a static instruction. The async context fetch happens
        // implicitly via the conversation messages that Page-Agent builds
        // (which include the user's task description).
        //
        // Phase 2 can wire this up properly by pre-fetching context on
        // tab change and caching it synchronously.
        const currentTab = useAppStore.getState().currentTab
        return `Current tab: ${currentTab}. Read what is visible on this page before taking any action.`
      },
    },

    // Disable the experimental script execution tool — Phase 1 is
    // read-only + simple form interactions only. No arbitrary JS eval.
    experimentalScriptExecutionTool: false,

    // Disable llms.txt fetching — our app doesn't serve one.
    experimentalLlmsTxt: false,

    // Step delay — keep small for snappy Phase 1 demo.
    stepDelay: 0.2,
  })

  return agentInstance
}

/**
 * Dispose the singleton agent and release DOM listeners.
 *
 * Safe to call multiple times. After disposal, the next `getAgent()` call
 * will create a fresh instance.
 */
export function disposeAgent(): void {
  if (agentInstance && !agentInstance.disposed) {
    agentInstance.dispose()
  }
  agentInstance = null
}

/**
 * Convenience: execute a one-shot task on the agent. Creates the singleton
 * if needed, runs the task, returns the ExecutionResult.
 *
 * Used by AiWorkspace's Agent Mode submit handler.
 */
export async function executeAgentTask(task: string) {
  const agent = await getAgent()
  return await agent.execute(task)
}

/**
 * Pre-fetch business context from Rust. Useful for displaying in the UI
 * ("Agent sees: 14 products, 2 drafts, on Catalog tab") before the user
 * submits a task.
 */
export async function fetchAgentContext(currentTab?: string) {
  return await invoke<{
    current_tab: string
    product_count: number
    draft_count: number
    ai_provider: string
    ai_model: string
    app_version: string
  }>('page_agent_get_context', { currentTab: currentTab ?? null })
}
