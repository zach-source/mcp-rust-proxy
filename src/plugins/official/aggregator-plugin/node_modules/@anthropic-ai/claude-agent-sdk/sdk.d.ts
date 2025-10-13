import type { Options as BaseOptions, Query, SDKUserMessage } from './sdkTypes.js';
export type AgentDefinition = {
    description: string;
    tools?: string[];
    prompt: string;
    model?: 'sonnet' | 'opus' | 'haiku' | 'inherit';
};
export type SettingSource = 'user' | 'project' | 'local';
export type Options = Omit<BaseOptions, 'customSystemPrompt' | 'appendSystemPrompt'> & {
    agents?: Record<string, AgentDefinition>;
    settingSources?: SettingSource[];
    systemPrompt?: string | {
        type: 'preset';
        preset: 'claude_code';
        append?: string;
    };
};
export declare function query(_params: {
    prompt: string | AsyncIterable<SDKUserMessage>;
    options?: Options;
}): Query;
export type { NonNullableUsage, ModelUsage, ApiKeySource, ConfigScope, McpStdioServerConfig, McpSSEServerConfig, McpHttpServerConfig, McpSdkServerConfig, McpSdkServerConfigWithInstance, McpServerConfig, McpServerConfigForProcessTransport, PermissionBehavior, PermissionUpdate, PermissionResult, PermissionRuleValue, CanUseTool, HookEvent, HookCallback, HookCallbackMatcher, BaseHookInput, PreToolUseHookInput, PostToolUseHookInput, NotificationHookInput, UserPromptSubmitHookInput, SessionStartHookInput, StopHookInput, SubagentStopHookInput, PreCompactHookInput, ExitReason, SessionEndHookInput, HookInput, AsyncHookJSONOutput, SyncHookJSONOutput, HookJSONOutput, PermissionMode, SlashCommand, ModelInfo, McpServerStatus, SDKMessageBase, SDKUserMessage, SDKUserMessageReplay, SDKAssistantMessage, SDKPermissionDenial, SDKResultMessage, SDKSystemMessage, SDKPartialAssistantMessage, SDKCompactBoundaryMessage, SDKMessage, Query, } from './sdkTypes.js';
export { HOOK_EVENTS, EXIT_REASONS, tool, createSdkMcpServer, AbortError, } from './sdkTypes.js';
