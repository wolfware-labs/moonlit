using System.ClientModel;
using OpenAI.Chat;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;

public interface IOpenAiClient
{
  Task<ClientResult<ChatCompletion>> CompleteChatAsync(params ChatMessage[] messages);
}
