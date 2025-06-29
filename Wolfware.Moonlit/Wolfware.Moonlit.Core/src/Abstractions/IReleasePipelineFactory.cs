using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Pipelines;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Defines a contract for creating instances of <see cref="ReleasePipeline"/> based on the provided
/// <see cref="ReleaseConfiguration"/>.
/// </summary>
public interface IReleasePipelineFactory
{
  /// <summary>
  /// Creates an instance of <see cref="ReleasePipeline"/> based on the provided <see cref="ReleaseConfiguration"/>.
  /// </summary>
  /// <param name="configuration">The release configuration used to initialize the pipeline, including plugins, stages, and variables.</param>
  /// <returns>A task that represents the asynchronous operation. The task result contains the created <see cref="ReleasePipeline"/>.</returns>
  Task<ReleasePipeline> Create(ReleaseConfiguration configuration);
}
