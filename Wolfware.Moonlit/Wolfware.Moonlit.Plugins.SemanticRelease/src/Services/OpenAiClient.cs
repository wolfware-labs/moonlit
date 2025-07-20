using System.ClientModel;
using System.Net;
using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using OpenAI.Chat;
using Polly;
using Polly.Retry;
using Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public partial class OpenAiClient : IOpenAiClient
{
  private readonly ILogger<OpenAiClient> _logger;
  private readonly SharedRetryCoordinator _retryCoordinator;
  private readonly OpenAiClientConfiguration _configuration;
  private readonly Lazy<ChatClient> _chatClient;
  private readonly AsyncRetryPolicy _retryPolicy;

  public OpenAiClient(ILogger<OpenAiClient> logger, IOptions<OpenAiClientConfiguration> configuration,
    SharedRetryCoordinator retryCoordinator)
  {
    this._logger = logger;
    this._configuration = configuration.Value;
    this._retryCoordinator = retryCoordinator;
    this._chatClient = new Lazy<ChatClient>(CreateChatClient);
    this._retryPolicy = this.CreateRetryPolicy();
  }

  public Task<ClientResult<ChatCompletion>> CompleteChatAsync(params ChatMessage[] messages)
  {
    return this._retryPolicy.ExecuteAsync(async () =>
    {
      await this._retryCoordinator.WaitIfRateLimitedAsync();
      return await this._chatClient.Value.CompleteChatAsync(messages);
    });
  }

  private ChatClient CreateChatClient()
  {
    if (string.IsNullOrWhiteSpace(this._configuration.Model))
    {
      throw new InvalidOperationException("AI Agent model is not configured.");
    }

    if (string.IsNullOrWhiteSpace(this._configuration.ApiKey))
    {
      throw new InvalidOperationException("AI Agent API key is not configured.");
    }

    return new ChatClient(this._configuration.Model, this._configuration.ApiKey);
  }

  private AsyncRetryPolicy CreateRetryPolicy()
  {
    return Policy
      .Handle<ClientResultException>(ex =>
        ex.Status == (int)HttpStatusCode.TooManyRequests || ex.Message.Contains("rate_limit_exceeded")
      )
      .WaitAndRetryAsync(
        retryCount: 5,
        sleepDurationProvider: (retryAttempt, context) =>
        {
          var delay = TimeSpan.FromSeconds(Math.Pow(2, retryAttempt));

          if (context.TryGetValue("retry-after-seconds", out var val) &&
              double.TryParse(val?.ToString(), out double seconds))
          {
            delay = TimeSpan.FromSeconds(seconds);
          }

          this._retryCoordinator.SetGlobalRetryAfter(delay);
          return delay;
        },
        onRetry: (exception, delay, attempt, context) =>
        {
          var retryAfter = OpenAiClient.ExtractRetryAfterSeconds(exception.Message);
          if (retryAfter != null)
          {
            context["retry-after-seconds"] = retryAfter.ToString();
            delay = TimeSpan.FromSeconds(retryAfter.Value);
          }

          this._logger.LogWarning(
            "[Retry {Attempt}] Rate limited by OpenAI API. Retrying in {Delay} seconds...",
            attempt, delay.TotalSeconds
          );
        });
  }

  private static double? ExtractRetryAfterSeconds(string message)
  {
    var match = OpenAiClient.GetRetryMessageRegex().Match(message);
    if (match.Success && double.TryParse(match.Groups[1].Value, out double seconds))
    {
      return seconds;
    }

    return null;
  }

  [GeneratedRegex("try again in ([0-9.]+)s", RegexOptions.IgnoreCase, "en-US")]
  private static partial Regex GetRetryMessageRegex();
}
