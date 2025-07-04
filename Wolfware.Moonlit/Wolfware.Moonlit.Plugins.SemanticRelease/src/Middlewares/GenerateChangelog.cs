using System.Diagnostics;
using System.Text.Json;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using OpenAI.Chat;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class GenerateChangelog : ReleaseMiddleware<GenerateChangelog.Configuration>
{
  public sealed class Configuration
  {
    public CommitMessage[] Commits { get; set; } = [];

    public string OpenAiKey { get; set; } = string.Empty;
  }

  public override async Task<MiddlewareResult> ExecuteAsync(PipelineContext context, Configuration configuration)
  {
    if (configuration.Commits.Length == 0)
    {
      context.Logger.LogWarning("No commits provided for changelogs generation.");
      return MiddlewareResult.Success();
    }

    var client = new ChatClient(model: "gpt-4o", apiKey: configuration.OpenAiKey);

    context.Logger.LogInformation("Generating changelogs for {CommitCount} commits.", configuration.Commits.Length);

    var jsonCommits = string.Join(",\n", configuration.Commits.Select(commit => JsonSerializer.Serialize(commit)));
    var prompt = $"""
                  Given the following list of Git commits in JSON format, generate a structured changelog in JSON format. 
                  Group entries under keys like 'features', 'bugfixes', 'documentation', and 'breakingChanges'. 
                  Each entry must include a 'sha' and a concise, well-written 'description'. 
                  Only include relevant user-facing or breaking changes (in conventional commit format). 
                  Skip internal, refactoring, or trivial commits unless marked as breaking.
                  Make sure to use the correct commit SHA for each entry.

                  Commits:
                  {jsonCommits}
                  """;

    var stopWatch = Stopwatch.StartNew();
    var completion = await client.CompleteChatAsync(
      new SystemChatMessage("You are a changelog generator that returns structured JSON output based on Git commits."),
      new SystemChatMessage(
        "Respond ONLY with the JSON object. Do not include any explanations, formatting, markdown or wrapping around the JSON."),
      new SystemChatMessage("JSON output should not contain any invalid characters."),
      new UserChatMessage(prompt)
    );

    if (completion.Value.Content.Count == 0)
    {
      context.Logger.LogError("Failed to generate changelogs. No content returned from OpenAI.");
      return MiddlewareResult.Failure("Failed to generate changelogs. No content returned from OpenAI.");
    }

    context.Logger.LogInformation("Changelogs generated successfully.");
    context.Logger.LogInformation("Changelogs generation took {ElapsedMilliseconds} ms.",
      stopWatch.ElapsedMilliseconds);

    var changelog = JsonSerializer.Deserialize<Dictionary<string, ChangelogEntry[]>>(completion.Value.Content[0].Text,
      JsonSerializerOptions.Web);
    if (changelog == null)
    {
      return MiddlewareResult.Failure("Failed to generate changelogs JSON.");
    }

    var hallucinationCheck = changelog.Values
      .SelectMany(entries => entries)
      .Any(entry => string.IsNullOrWhiteSpace(entry.Description) || string.IsNullOrWhiteSpace(entry.Sha) ||
                    configuration.Commits.All(c => c.Sha != entry.Sha));

    if (!hallucinationCheck)
    {
      return MiddlewareResult.Success(output =>
      {
        output.Add("Changelog", changelog);
      });
    }

    context.Logger.LogWarning("Generated changelogs may contain hallucinations or invalid entries.");
    return MiddlewareResult.Failure("Generated changelogs may contain hallucinations or invalid entries.");
  }
}
