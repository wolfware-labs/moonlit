using System.Net;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using NuGet.Common;
using NuGet.Protocol;
using NuGet.Protocol.Core.Types;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Dotnet.Middlewares;

public sealed class PushPackage : ReleaseMiddleware<PublishPackageConfiguration>
{
  private readonly ILogger<PushPackage> _logger;
  private readonly DotnetConfiguration _globalConfiguration;

  public PushPackage(ILogger<PushPackage> logger, IOptions<DotnetConfiguration> globalConfiguration)
  {
    _logger = logger;
    _globalConfiguration = globalConfiguration.Value;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    PublishPackageConfiguration configuration)
  {
    if (!File.Exists(configuration.Package))
    {
      return MiddlewareResult.Failure($"NuGet package file not found at path: {configuration.Package}");
    }

    var source = string.IsNullOrWhiteSpace(configuration.Source)
      ? _globalConfiguration.NugetSource
      : configuration.Source;
    if (string.IsNullOrWhiteSpace(source))
    {
      return MiddlewareResult.Failure("NuGet source is not specified in both global and local configuration.");
    }

    var apiKey = string.IsNullOrWhiteSpace(configuration.ApiKey)
      ? _globalConfiguration.NugetApiKey
      : configuration.ApiKey;
    if (string.IsNullOrWhiteSpace(apiKey))
    {
      return MiddlewareResult.Failure("NuGet API key is not specified in both global and local configuration.");
    }

    var repository = Repository.Factory.GetCoreV3(source);
    var packageUpdateResource = await repository.GetResourceAsync<PackageUpdateResource>();

    try
    {
      this._logger.LogInformation("Pushing package {PackagePath} to source {Source}",
        Path.GetFileNameWithoutExtension(configuration.Package), source);
      await packageUpdateResource.Push(
        packagePaths: [configuration.Package],
        symbolSource: string.Empty,
        timeoutInSecond: 30,
        disableBuffering: false,
        getApiKey: _ => apiKey,
        getSymbolApiKey: _ => null,
        noServiceEndpoint: false,
        skipDuplicate: false,
        symbolPackageUpdateResource: null,
        allowInsecureConnections: false,
        log: NullLogger.Instance
      );
      this._logger.LogInformation("Package published successfully");
      return MiddlewareResult.Success();
    }
    catch (HttpRequestException e)
    {
      if (e.StatusCode is HttpStatusCode.Unauthorized or HttpStatusCode.Forbidden)
      {
        this._logger.LogError("Failed to push package {PackagePath} to source {Source} due to authentication error",
          Path.GetFileNameWithoutExtension(configuration.Package), source);
        return MiddlewareResult.Failure(
          "Failed to push package: Authentication error. Please check your API key and permissions.");
      }

      this._logger.LogError(e, "Failed to push package {PackagePath} to source {Source} due to network error",
        Path.GetFileNameWithoutExtension(configuration.Package), source);
      return MiddlewareResult.Failure($"Failed to push package: Network error - {e.Message}");
    }
    catch (Exception e)
    {
      this._logger.LogError(e, "Failed to push package {PackagePath} to source {Source}",
        Path.GetFileNameWithoutExtension(configuration.Package), source);
      return MiddlewareResult.Failure($"Failed to push package: {e.Message}");
    }
  }
}
