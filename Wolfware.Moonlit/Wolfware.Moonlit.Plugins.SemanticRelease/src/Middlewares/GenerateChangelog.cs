using System.Diagnostics;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using OpenAI.Chat;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Extensions;
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
      this._logger.LogWarning("No commits provided for changelog generation.");
      return MiddlewareResult.Success();
    }

    var client = new ChatClient(model: "gpt-4o", apiKey: configuration.OpenAiKey);

    this._logger.LogInformation("Generating changelog for {CommitCount} commits.", configuration.Commits.Length);
    try
    {
      var userFacingCommits = await FilterOutNonUserFacingCommits(client, configuration.Commits);
      if (userFacingCommits.Length == 0)
      {
        this._logger.LogInformation("No user-facing commits found. Skipping changelog generation.");
        return MiddlewareResult.Success();
      }

      this._logger.LogInformation("Filtered down to {UserFacingCommitCount} user-facing commits.",
        userFacingCommits.Length);
      var changelogEntries = await RefineCommitsIntoChangelogEntries(client, userFacingCommits);
      if (changelogEntries.Length == 0)
      {
        this._logger.LogInformation("No changelog entries generated from user-facing commits.");
        return MiddlewareResult.Success();
      }

      this._logger.LogInformation("Generated {ChangelogEntryCount} changelog entries.", changelogEntries.Length);
      var categorizedChangelog = await CategorizeChangelogEntries(client, changelogEntries);


      return MiddlewareResult.Success(output =>
      {
        output.Add("Categories", categorizedChangelog);
      });
    }
    catch (Exception ex)
    {
      this._logger.LogError(ex, "An error occurred while generating changelogs.");
      return MiddlewareResult.Failure(ex.Message);
    }
  }

  private async Task<CommitMessage[]> FilterOutNonUserFacingCommits(ChatClient client, CommitMessage[] commits)
  {
    var responses = await commits.ParallelForEachBatch(15, async batch =>
    {
      var commitsJson = string.Join(",",
        batch.Select(commit => JsonSerializer.Serialize(new {commit.Sha, commit.Message})));
      var completion = await client.CompleteChatAsync(
        new SystemChatMessage(
          "You are an expert at generating accurate and user-friendly changelogs based on commit messages."),
        new UserChatMessage(
          $"Please review the following list of commit messages in JSON format containing the commit SHA and the commit Message. Return only the SHA of those that are relevant to end users and should be included in a changelog. Ignore any commits related to internal documentation, refactoring, or non-user-facing changes. The commits are following conventional commits specification. The commit list is as follows: {commitsJson}. Return the filtered list as an array of strings. DO NOT wrap the array in markdown or any other formatting. Just return the JSON array directly.")
      );

      if (completion.Value.Content.Count == 0)
      {
        this._logger.LogError("Failed to filter commits. No content returned from OpenAI.");
        throw new InvalidOperationException("Failed to filter commits. No content returned from OpenAI.");
      }

      var filteredCommitsJson = completion.Value.Content[0].Text.Trim();
      if (!string.IsNullOrWhiteSpace(filteredCommitsJson))
      {
        var filteredShas = JsonSerializer.Deserialize<string[]>(filteredCommitsJson, JsonSerializerOptions.Web)
                           ?? throw new InvalidOperationException("Failed to deserialize filtered commits JSON.");
        return filteredShas
          .Select(sha => batch.First(commit => commit.Sha.Equals(sha, StringComparison.OrdinalIgnoreCase)))
          .ToArray();
      }

      this._logger.LogWarning("No user-facing commits found in the provided list.");
      return [];
    });

    return responses.SelectMany(response => response).ToArray();
  }

  private async Task<ChangelogEntry[]> RefineCommitsIntoChangelogEntries(ChatClient client, CommitMessage[] commits)
  {
    var responses = await commits.ParallelForEachBatch(15, async batch =>
    {
      var commitsJson = string.Join(",",
        batch.Select(commit => JsonSerializer.Serialize(new {commit.Sha, commit.Message})));
      var completion = await client.CompleteChatAsync(
        new SystemChatMessage(
          "You are an expert at generating accurate and user-friendly changelogs based on commit messages."),
        new UserChatMessage(
          $"Please transform the following list of commit messages in JSON format containing the commit SHA and the commit Message, replacing the Message for a more user-friendly changelog entry. Ensure the entries are clear, concise, and highlight the value to the end user. Keep in mind that the original commits are following conventional commits specification. The commit list is as follows: {commitsJson}. Return the filtered list in JSON format as an array of objects with the fields 'Sha' and 'Description' where the Sha field contains the commit SHA and the Description field contains the refined message. DO NOT wrap the array in markdown or any other formatting. Just return the JSON array directly.")
      );
      if (completion.Value.Content.Count == 0)
      {
        this._logger.LogError("Failed to generate changelog entries. No content returned from OpenAI.");
        throw new InvalidOperationException("Failed to generate changelog entries. No content returned from OpenAI.");
      }

      var changelogJson = completion.Value.Content[0].Text.Trim();
      if (string.IsNullOrWhiteSpace(changelogJson))
      {
        this._logger.LogWarning("No changelog entries generated from the provided commits.");
        return [];
      }

      var changelogEntries = JsonSerializer.Deserialize<ChangelogEntry[]>(changelogJson, JsonSerializerOptions.Web);
      if (changelogEntries == null)
      {
        throw new InvalidOperationException("Failed to deserialize changelog entries JSON.");
      }

      if (batch.Length != changelogEntries.Length)
      {
        throw new InvalidOperationException(
          $"Mismatch in number of commits ({batch.Length}) and generated changelog entries ({changelogEntries.Length}).");
      }

      if (changelogEntries.Any(x => string.IsNullOrWhiteSpace(x.Sha) ||
                                    string.IsNullOrWhiteSpace(x.Description) ||
                                    batch.All(c => !c.Sha.Equals(x.Sha, StringComparison.OrdinalIgnoreCase)))
         )
      {
        this._logger.LogError("Generated changelog entries contain invalid data.");
        throw new InvalidOperationException("Generated changelog entries contain invalid data.");
      }

      return changelogEntries;
    });

    return responses.SelectMany(response => response).ToArray();
  }

  private async Task<ChangelogCategory[]> CategorizeChangelogEntries(ChatClient client, ChangelogEntry[] entries)
  {
    var entriesJson = string.Join(",", entries.Select(commit => JsonSerializer.Serialize(commit)));
    var completion = await client.CompleteChatAsync(
      new SystemChatMessage(
        "You are an expert at generating and organizing changelogs."),
      new UserChatMessage(
        "Please take the following list of refined changelog entries and categorize each entry into standard changelog categories." +
        "Based on the categorized changelog entries, please generate a brief summary for each category and provide a heading for each section." +
        "Include an emoji in text format for each category to enhance readability." +
        "Return the final result in JSON format, with each category having the following fields: name, icon, summary and entries" +
        "DO NOT wrap the array in markdown or any other formatting. Just return the JSON array directly." +
        $"The entries are as follows: {entriesJson}."
      )
    );
    if (completion.Value.Content.Count == 0)
    {
      this._logger.LogError("Failed to categorize changelog entries. No content returned from OpenAI.");
      throw new InvalidOperationException("Failed to categorize changelog entries. No content returned from OpenAI.");
    }

    var categorizedChangelogJson = completion.Value.Content[0].Text.Trim();
    if (string.IsNullOrWhiteSpace(categorizedChangelogJson))
    {
      this._logger.LogWarning("No categorized changelog entries generated from the provided entries.");
      return [];
    }

    var categorizedChangelog =
      JsonSerializer.Deserialize<ChangelogCategory[]>(categorizedChangelogJson, JsonSerializerOptions.Web);
    if (categorizedChangelog == null)
    {
      this._logger.LogError("Failed to deserialize categorized changelog JSON.");
      throw new InvalidOperationException("Failed to deserialize categorized changelog JSON.");
    }

    this._logger.LogInformation("Categorized changelog entries into {CategoryCount} categories.",
      categorizedChangelog.Length);
    return categorizedChangelog;
  }
}
