using System.Text.Json;
using Microsoft.Extensions.Logging;
using OpenAI.Chat;
using Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Extensions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class AiAgent : IAiAgent
{
  private readonly ILogger<AiAgent> _logger;
  private readonly IOpenAiClient _openAiClient;

  public AiAgent(ILogger<AiAgent> logger, IOpenAiClient openAiClient)
  {
    this._logger = logger;
    this._openAiClient = openAiClient;
  }

  public async Task<ConventionalCommit[]> FilterOutNonUserFacingCommits(ConventionalCommit[] commits)
  {
    var responses = await commits.ParallelForEachBatch(15, async batch =>
    {
      var commitsJson = string.Join(",",
        batch.Select(commit => JsonSerializer.Serialize(new {commit.Sha, FullMessage = commit.RawMessage})));
      var completion = await this._openAiClient.CompleteChatAsync(
        new SystemChatMessage(
          "You are an AI assistant that analyzes Git commit messages and selects those relevant to end users for a changelog. " +
          "You follow the conventional commits specification strictly and never include internal or non-user-facing changes unless explicitly relevant. " +
          "You always return a clean, valid JSON array of commit SHAs and never include explanations or additional text. " +
          "If no commits are relevant, return an empty array: []."
        ),
        new UserChatMessage(
          "You are a strict JSON-generating assistant. Your job is to review a list of Git commit messages formatted in JSON and return only the relevant ones for a public changelog.\n\n" +
          "Rules:\n" +
          "1. Only include commits relevant to end users (e.g., new features, bug fixes, breaking changes).\n" +
          "2. Ignore internal changes like refactor, test, chore, ci, docs, etc., unless they are user-facing.\n" +
          "3. Follow the conventional commits specification to determine relevance.\n\n" +
          "Output:\n" +
          "Return a **strictly valid** JSON array of commit SHAs. Do **not** include any extra text or comments.\n\n" +
          "Example output:\n" +
          "[\n  \"1a2b3c4d\",\n  \"5e6f7g8h\"\n]\n\n" +
          "Commit list (in JSON format) is between the triple backticks below:\n" +
          "```\n" +
          $"{commitsJson}\n" +
          "```"
        )
      );

      if (completion.Value.Content.Count == 0)
      {
        this._logger.LogError("Failed to filter commits. No content returned from AI agent.");
        throw new InvalidOperationException("Failed to filter commits. No content returned from AI Agent.");
      }

      var filteredCommitsJson = completion.Value.Content[0].Text.Trim();
      if (string.IsNullOrWhiteSpace(filteredCommitsJson))
      {
        this._logger.LogWarning("No user-facing commits found in the provided list.");
        return [];
      }

      var filteredShas = JsonSerializer.Deserialize<string[]>(filteredCommitsJson, JsonSerializerOptions.Web)
                         ?? throw new InvalidOperationException("Failed to deserialize filtered commits JSON.");
      return filteredShas
        .Select(sha => batch.First(commit => commit.Sha.Equals(sha, StringComparison.OrdinalIgnoreCase)))
        .ToArray();
    });

    return responses.SelectMany(response => response).ToArray();
  }

  public async Task<ConventionalCommit[]> RefineCommitsSummary(ConventionalCommit[] commits)
  {
    var responses = await commits.ParallelForEachBatch(15, async batch =>
    {
      var commitsJson = string.Join(",",
        batch.Select(commit => JsonSerializer.Serialize(new {commit.Sha, commit.RawMessage})));
      var completion = await this._openAiClient.CompleteChatAsync(
        new SystemChatMessage(
          "You are a professional changelog writer. You take Git commit messages that follow the conventional commits specification " +
          "and rewrite them into clear, user-friendly descriptions suitable for inclusion in a product changelog. " +
          "You do not remove any items. For each commit, keep the original SHA and generate a rewritten message that non-technical users can understand. " +
          "Avoid technical jargon unless it is user-facing. Return a valid JSON array where each item contains the original commit SHA and the rewritten message."
        ),
        new UserChatMessage(
          "Please rewrite the following commit messages into user-friendly changelog entries.\n\n" +
          "Each commit message follows the conventional commits specification. Do not exclude any commits. " +
          "For each one, include:\n" +
          "- The original `sha`.\n" +
          "- A `message` field with a clear, readable summary suitable for end users.\n\n" +
          "Return a valid JSON array like this:\n" +
          "[\n" +
          "  { \"sha\": \"1a2b3c4d\", \"summary\": \"Improved the login experience with faster load times.\" },\n" +
          "  { \"sha\": \"5e6f7g8h\", \"summary\": \"Fixed an issue where notifications wouldn’t appear.\" }\n" +
          "]\n\n" +
          "Here is the input commit list in JSON format:\n" +
          "```\n" +
          $"{commitsJson}\n" +
          "```"
        )
      );

      if (completion.Value.Content.Count == 0)
      {
        var errorMessage = "Failed to refine commits summary. No content returned from AI Agent.";
        this._logger.LogError(errorMessage);
        throw new InvalidOperationException(errorMessage);
      }

      var summariesJson = completion.Value.Content[0].Text.Trim();
      if (string.IsNullOrWhiteSpace(summariesJson))
      {
        var errorMessage = "AI Agent was not able to refine commit summaries. Empty response.";
        this._logger.LogError(errorMessage);
        throw new InvalidOperationException(errorMessage);
      }

      var commitSummaries = JsonSerializer.Deserialize<CommitSummary[]>(summariesJson, JsonSerializerOptions.Web);
      if (commitSummaries == null)
      {
        throw new InvalidOperationException("Failed to deserialize commit summaries returned by AI agent.");
      }

      if (batch.Length != commitSummaries.Length)
      {
        throw new InvalidOperationException(
          $"Mismatch in number of commits ({batch.Length}) and generated changelog entries ({commitSummaries.Length}).");
      }

      var processedCommits = commitSummaries.ToDictionary(s => s.Sha, StringComparer.OrdinalIgnoreCase);

      foreach (var commit in batch)
      {
        if (processedCommits.TryGetValue(commit.Sha, out var summary))
        {
          commit.Summary = summary.Summary;
        }
        else
        {
          this._logger.LogError("Missing summary for commit {Sha} in AI agent response.", commit.Sha);
          throw new InvalidOperationException($"Missing summary for commit {commit.Sha} in AI agent response.");
        }
      }

      return batch;
    });

    return responses.SelectMany(response => response).ToArray();
  }
}
