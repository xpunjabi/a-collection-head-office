/**
 * v0.23.0 — Page-Agent Bridge (Phase 1: Foundation)
 * ================================================
 *
 * createTauriFetch — a `customFetch` implementation that intercepts every
 * HTTP call Page-Agent's built-in `OpenAIClient` would make and routes it
 * through the existing Rust `call_ai_provider` pipeline via the new Tauri
 * command `page_agent_invoke`.
 *
 * ARCHITECTURAL INVARIANTS (see commands/mod.rs `page_agent_invoke`):
 *   1. ONE AI config — provider/key/model come from existing Settings.
 *   2. ONE request pipeline — we never call Gemini/OpenAI/Claude directly.
 *      customFetch → Tauri invoke → Rust `call_ai_provider` → provider API.
 *   3. NO second API key — bridge receives NO credentials. Rust holds them.
 *   4. NO local models — Rust side rejects the "local" provider for this
 *      path (it routes through call_ai_provider which doesn't support the
 *      Ollama-shape local LLM for tool calling).
 *
 * Why customFetch (not custom LLMClient)?
 *   PageAgentCore's `#llm` field is private — we cannot replace its LLM
 *   client post-construction. But Page-Agent's `LLMConfig.customFetch` is
 *   the officially-supported extension point. We let the built-in
 *   `OpenAIClient` handle all OpenAI wire-format parsing (tool_calls,
 *   finish_reason, etc.) and just intercept the actual HTTP call.
 *
 * If Page-Agent is ever removed from the project, delete this file and
 * pageAgentAdapter.ts — nothing else in the codebase depends on them.
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * Shape of the request body OpenAIClient sends to {baseURL}/chat/completions.
 * We declare only the fields we read — others pass through untouched.
 */
interface OpenAIRequestBody {
  messages: Array<{
    role: string
    content?: string | null
    tool_calls?: unknown
    tool_call_id?: string
  }>
  tools?: Array<{
    type: 'function'
    function: {
      name: string
      description?: string
      parameters?: unknown // JSON Schema
    }
  }>
  tool_choice?: unknown
  model?: string
}

/**
 * Wire format matching what the Rust `page_agent_invoke` command expects.
 * Mirrors the `PageAgentToolDef` struct in commands/mod.rs.
 */
interface RustToolDef {
  name: string
  description: string
  parameters: unknown
}

/**
 * Wire format matching what Rust `page_agent_invoke` returns — OpenAI
 * Chat Completions response shape.
 */
interface OpenAIChatCompletionResponse {
  choices: Array<{
    message: {
      role: string
      content?: string | null
      tool_calls?: Array<{
        id: string
        type: 'function'
        function: { name: string; arguments: string }
      }>
    }
    finish_reason: string
  }>
  usage: {
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }
}

/**
 * Build a `customFetch` function for Page-Agent's LLMConfig.
 *
 * The returned function has the same signature as `globalThis.fetch` so
 * Page-Agent's OpenAIClient can use it transparently. It:
 *   1. Parses the OpenAI-format request body sent by OpenAIClient
 *   2. Extracts messages + tools + tool_choice
 *   3. Calls the Rust `page_agent_invoke` Tauri command
 *   4. Wraps the Rust response as a fetch-like Response object
 *
 * Errors are wrapped in a fetch-like Response with status 500 so
 * OpenAIClient's existing error handling kicks in.
 */
export function createTauriFetch(): typeof globalThis.fetch {
  return async (
    _input: URL | RequestInfo,
    init?: RequestInit,
  ): Promise<Response> => {
    // 1. Parse the request body OpenAIClient constructed.
    let requestBody: OpenAIRequestBody
    try {
      const bodyStr = typeof init?.body === 'string'
        ? init.body
        : init?.body instanceof Uint8Array
          ? new TextDecoder().decode(init.body)
          : ''
      requestBody = JSON.parse(bodyStr) as OpenAIRequestBody
    } catch (e) {
      return new Response(
        JSON.stringify({ error: { message: `Bridge: failed to parse request body: ${(e as Error).message}` } }),
        { status: 400, headers: { 'Content-Type': 'application/json' } },
      )
    }

    // 2. Convert OpenAI tools format → Rust wire format.
    const toolsWire: RustToolDef[] = (requestBody.tools ?? []).map((t) => ({
      name: t.function.name,
      description: t.function.description ?? '',
      parameters: t.function.parameters ?? { type: 'object', properties: {} },
    }))

    // 3. Extract tool_choice if it's a named function call.
    let toolChoiceName: string | null = null
    const tc = requestBody.tool_choice
    if (tc && typeof tc === 'object' && 'function' in tc) {
      const fn = (tc as { function: { name?: string } }).function
      if (fn && typeof fn.name === 'string') {
        toolChoiceName = fn.name
      }
    }

    // 4. Strip tool_calls/tool_call_id from messages — Rust side treats
    //    them as opaque text content for Phase 1. We keep the role + content
    //    text only.
    const messagesWire = requestBody.messages.map((m) => ({
      role: m.role,
      content: m.content ?? '',
    }))

    // 5. Call the Rust bridge. Tauri serialises args + deserialises the
    //    response automatically.
    let rustResponse: OpenAIChatCompletionResponse
    try {
      rustResponse = await invoke<OpenAIChatCompletionResponse>(
        'page_agent_invoke',
        {
          messages: messagesWire,
          tools: toolsWire,
          toolChoiceName,
        },
      )
    } catch (e) {
      // Tauri command returned Err — wrap as HTTP 500 so OpenAIClient's
      // existing error handling surfaces it to the user.
      return new Response(
        JSON.stringify({ error: { message: `Rust bridge: ${e as string}` } }),
        { status: 500, headers: { 'Content-Type': 'application/json' } },
      )
    }

    // 6. Return as a fetch-like Response. OpenAIClient will JSON.parse
    //    the body and extract tool_calls.
    return new Response(JSON.stringify(rustResponse), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    })
  }
}
