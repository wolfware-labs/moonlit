using System.Diagnostics;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using OpenAI.Chat;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class GenerateChangelog : ReleaseMiddleware<GenerateChangelog.Configuration>
{
  private readonly ILogger<GenerateChangelog> _logger;

  public sealed class Configuration
  {
    public CommitMessage[] Commits { get; set; } = [];

    public string OpenAiKey { get; set; } = string.Empty;
  }

  public GenerateChangelog(ILogger<GenerateChangelog> logger)
  {
    _logger = logger;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    if (configuration.Commits.Length == 0)
    {
      this._logger.LogWarning("No commits provided for changelogs generation.");
      return MiddlewareResult.Success();
    }

    var client = new ChatClient(model: "gpt-4o", apiKey: configuration.OpenAiKey);

    this._logger.LogInformation("Generating changelogs for {CommitCount} commits.", configuration.Commits.Length);

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
      this._logger.LogError("Failed to generate changelogs. No content returned from OpenAI.");
      return MiddlewareResult.Failure("Failed to generate changelogs. No content returned from OpenAI.");
    }

    this._logger.LogInformation("Changelogs generated successfully.");
    this._logger.LogInformation("Changelogs generation took {ElapsedMilliseconds} ms.",
      stopWatch.ElapsedMilliseconds);

    var entries = JsonSerializer.Deserialize<Dictionary<string, ChangelogEntry[]>>(completion.Value.Content[0].Text,
      JsonSerializerOptions.Web);
    if (entries == null)
    {
      return MiddlewareResult.Failure("Failed to generate changelogs JSON.");
    }

    var hallucinationCheck = entries.Values
      .SelectMany(entries => entries)
      .Any(entry => string.IsNullOrWhiteSpace(entry.Description) || string.IsNullOrWhiteSpace(entry.Sha) ||
                    configuration.Commits.All(c => c.Sha != entry.Sha));

    if (!hallucinationCheck)
    {
      return MiddlewareResult.Success(output =>
      {
        output.Add("Entries", entries);
      });
    }

    this._logger.LogWarning("Generated changelogs may contain hallucinations or invalid entries.");
    return MiddlewareResult.Failure("Generated changelogs may contain hallucinations or invalid entries.");
  }
}
