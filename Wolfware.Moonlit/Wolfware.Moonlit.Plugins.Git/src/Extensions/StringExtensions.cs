namespace Wolfware.Moonlit.Plugins.Git.Extensions;

internal static class StringExtensions
{
  public static string GetGitFolderPath(this string workingDirectory)
  {
    var originalWorkingDirectory = workingDirectory;
    while (true)
    {
      if (Directory.Exists(Path.Combine(workingDirectory, ".git")))
      {
        return Path.Combine(workingDirectory, ".git");
      }

      var parentDirectory = Directory.GetParent(workingDirectory);
      if (parentDirectory == null)
      {
        throw new InvalidOperationException(
          $"No Git repository found in the working directory or any of its parent directories: {originalWorkingDirectory}"
        );
      }

      workingDirectory = parentDirectory.FullName;
    }
  }
}
