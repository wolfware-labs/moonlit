using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using OpenAI.Chat;
using Semver;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class GenerateChangelogs : IReleaseMiddleware
{
  public async Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context);
    ArgumentNullException.ThrowIfNull(configuration);

    var config = configuration.GetRequired<GenerateChangelogsConfiguration>();
    if (config.Commits.Length == 0)
    {
      context.Logger.LogWarning("No commits provided for changelogs generation.");
      return MiddlewareResult.Success();
    }

    var client = new ChatClient(model: "gpt-4o", apiKey: config.OpenAiKey);

    context.Logger.LogInformation("Generating changelogs for {CommitCount} commits.", config.Commits.Length);

    var completion = await client.CompleteChatAsync(
      new SystemChatMessage("You are a helpful assistant that generates changelogs based on commit messages."),
      new SystemChatMessage(
        "The changelog should be in markdown format and include the type of change (e.g., feature, fix, chore) and a brief description for each commit."),
      new SystemChatMessage(
        "You will be provided with a list of commits. Please generate a changelog in markdown format that summarizes these commits."
      ),
      new SystemChatMessage(
        "Include emojis to enhance the readability of the changelog. Use appropriate emojis for each type of change."
      ),
      new UserChatMessage(
        $"Generate a changelog for the following commits:\n{string.Join("\n", config.Commits.ToString())}")
    );

    if (completion.Value.Content.Count == 0)
    {
      context.Logger.LogError("Failed to generate changelogs. No content returned from OpenAI.");
      return MiddlewareResult.Failure("Failed to generate changelogs. No content returned from OpenAI.");
    }

    context.Logger.LogInformation("Changelogs generated successfully.");

    return MiddlewareResult.Success(output =>
    {
      output.Add("Changelogs", completion.Value.Content[0].Text.Trim());
    });
  }
}
