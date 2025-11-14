# Active Tasks

**Last Updated**: November 13, 2025

## Current Sprint

No active sprint tasks - v0.2.0 released successfully

## Planned Features

### PackageManager API Improvements

- [ ] Add `is_installed(name: &str) -> Result<bool>` helper
- [ ] Add `install_multiple(names: &[&str]) -> Result<Vec<InstallResult>>` batch op
- [ ] Add `upgrade_multiple(names: &[&str]) -> Result<Vec<UpgradeResult>>` batch op
- [ ] Optimize `outdated()` for parallel API queries (40s → ~10s possible)

### Cask Support

- [ ] Wrap low-level `Cask` type in PackageManager
- [ ] Add `install_cask()` operation
- [ ] Add `upgrade_cask()` operation
- [ ] Test with common applications

## Phase 5: Source Builds (Future)

- [ ] Evaluate Ruby embedding options (`magnus` vs `rutie` vs others)
- [ ] Design formula execution interface
- [ ] Implement source build support for remaining ~5% formulae
- [ ] Test on uncommon packages

## Documentation

- [ ] Clarify library vs CLI usage in README
- [ ] Add migration guide for downstream projects
- [ ] Document caching strategy
- [ ] Add troubleshooting section

## Testing

- [ ] Test library API with Cutler integration
- [ ] Real-world validation on different Mac models
- [ ] Performance profiling on varying system sizes
- [ ] Edge case testing (interrupted operations, corrupted Cellar, etc.)

## Non-Urgent

- [ ] Performance: Parallelize outdated() queries
- [ ] Performance: Batch dependency lookups
- [ ] UX: Add progress callbacks for long operations
- [ ] UX: Better error messages for network failures
