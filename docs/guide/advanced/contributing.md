# Contributing to Moonlit

Thank you for your interest in contributing to Moonlit! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. We expect all contributors to be respectful and considerate of others.

## How to Contribute

There are many ways to contribute to Moonlit:

1. **Reporting Bugs**: If you find a bug, please create an issue in our GitHub repository with a detailed description.
2. **Suggesting Enhancements**: Have an idea for a new feature? Submit an issue with the enhancement tag.
3. **Writing Documentation**: Help improve our documentation by fixing errors or adding examples.
4. **Contributing Code**: Submit pull requests with bug fixes or new features.
5. **Sharing Examples**: Add new recipes to the cookbook section.

## Getting Started

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/your-username/moonlit.git
   cd moonlit
   ```
3. Add the original repository as an upstream remote:
   ```bash
   git remote add upstream https://github.com/wolfware-labs/moonlit.git
   ```
4. Install dependencies:
   ```bash
   dotnet restore
   ```

### Development Workflow

1. Create a new branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes
3. Run tests to ensure your changes don't break existing functionality:
   ```bash
   dotnet test
   ```
4. Commit your changes with a descriptive commit message:
   ```bash
   git commit -m "Add feature: your feature description"
   ```
5. Push your branch to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```
6. Create a pull request from your fork to the main repository

## Pull Request Guidelines

When submitting a pull request, please:

1. Include a clear description of the changes
2. Link to any related issues
3. Update documentation if necessary
4. Add or update tests as appropriate
5. Follow the existing code style and conventions
6. Ensure all tests pass

## Cookbook Contributions

We especially welcome contributions to the cookbook section. If you have a useful workflow or recipe:

1. Create a new markdown file in the `cookbook` directory
2. Follow the existing format of other cookbook entries
3. Include:
   - A clear title and description
   - Prerequisites
   - Step-by-step instructions
   - Code examples
   - Explanation of how the solution works
4. Submit a pull request with your new recipe

### Cookbook Entry Template

```markdown
---
title: Your Recipe Title
description: A brief description of what this recipe accomplishes
---

# Your Recipe Title

Brief introduction explaining what this recipe does and why it's useful.

## Prerequisites

- List of requirements
- Tools needed
- Any setup required

## Configuration

    # Your configuration example in YAML format
    name: example
    version: 1.0.0
    settings:
      key: value

## Explanation

Detailed explanation of how the solution works.

## Next Steps

- Suggestions for extending the recipe
- Related recipes or documentation
```

## Documentation Style Guide

When writing documentation:

1. Use clear, concise language
2. Structure content with headings and subheadings
3. Use code blocks for examples
4. Include explanations for complex concepts
5. Link to related documentation when appropriate

## Release Process

Moonlit follows semantic versioning. The release process is managed by the core team, but contributors should be aware of the versioning conventions:

- **Major version**: Breaking changes
- **Minor version**: New features without breaking changes
- **Patch version**: Bug fixes and minor improvements

## Getting Help

If you need help with contributing:

- Join our community chat
- Ask questions in GitHub issues
- Reach out to the maintainers

Thank you for contributing to Moonlit!
