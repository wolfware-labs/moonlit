using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using NuGet.Common;
using NuGet.Protocol;
using NuGet.Protocol.Core.Types;
using Wolfware.Moonlit.Plugins.Nuget.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Nuget.Middlewares;

public sealed class PushPackage : ReleaseMiddleware<PublishPackageConfiguration>
{
  private readonly ILogger<PushPackage> _logger;
  private readonly NugetConfiguration _globalConfiguration;

  public PushPackage(ILogger<PushPackage> logger, IOptions<NugetConfiguration> globalConfiguration)
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
      ? _globalConfiguration.Source
      : configuration.Source;
    if (string.IsNullOrWhiteSpace(source))
    {
      return MiddlewareResult.Failure("NuGet source is not specified in both global and local configuration.");
    }

    var apiKey = string.IsNullOrWhiteSpace(configuration.ApiKey)
      ? _globalConfiguration.ApiKey
      : configuration.ApiKey;
    if (string.IsNullOrWhiteSpace(apiKey))
    {
      return MiddlewareResult.Failure("NuGet API key is not specified in both global and local configuration.");
    }

    var repository = Repository.Factory.GetCoreV3(source);
    var packageUpdateResource = await repository.GetResourceAsync<PackageUpdateResource>();

    await packageUpdateResource.Push(
      packagePaths: [configuration.Package],
      symbolSource: string.Empty,
      timeoutInSecond: 30,
      disableBuffering: false,
      getApiKey: _ => apiKey,
      getSymbolApiKey: _ => null,
      noServiceEndpoint: false,
      skipDuplicate: true,
      symbolPackageUpdateResource: null,
      allowInsecureConnections: false,
      log: NullLogger.Instance
    );
    this._logger.LogInformation("Package published successfully: {PackagePath}", configuration.Package);
    return MiddlewareResult.Success();
  }
}
