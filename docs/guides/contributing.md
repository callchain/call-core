# Contributing Guide

Thank you for your interest in contributing to Call-Core! This guide covers the development workflow, coding standards, and best practices.

## Getting Started

### Prerequisites

- **Rust 1.70+** - Install via [rustup](https://rustup.rs/)
- **Git** - Version control
- **OpenSSL 3.0+** - Cryptographic library
- **CMake 3.16+** - Build system

### Clone Repository

```bash
git clone https://github.com/callchain/call-core.git
cd call-core
```

### Build

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (slower compile, faster runtime)
cargo build --release

# Run tests
cargo test --all
```

## Development Workflow

### 1. Create Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/changes

### 2. Make Changes

- Write code following our [Coding Standards](#coding-standards)
- Add tests for new functionality
- Update documentation as needed

### 3. Test

```bash
# Run all tests
cargo test --all

# Run specific test
cargo test test_name

# Run with output
cargo test --all -- --nocapture

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy --all-targets --all-features

# Check documentation
cargo doc --no-deps
```

### 4. Commit

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

<body>

<footer>
```

Types:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Formatting (no code change)
- `refactor` - Code refactoring
- `perf` - Performance improvement
- `test` - Test additions/changes
- `chore` - Build/tooling changes

Examples:
```
feat(consensus): add weighted validator voting

Implement weighted consensus calculation allowing validators
to have different voting weights based on stake or reputation.

Closes: #123
```

```
fix(tx): correct sequence number check in preclaim

The preclaim phase was not properly checking sequence numbers
for multi-signed transactions. This fix ensures the sequence
is validated against the signer list.

Fixes: #456
```

### 5. Submit Pull Request

1. Push branch to GitHub
2. Create Pull Request against `main`
3. Fill out PR template
4. Request review from maintainers

## Project Structure

```
call-core/
├── crates/
│   ├── primitives/       # Core data types
│   ├── serialization/    # STObject format
│   ├── crypto/          # Cryptography
│   ├── shamap/          # Merkle tree
│   ├── protocol/        # Ledger, transactions
│   ├── consensus/       # BFT consensus
│   ├── network/         # P2P networking
│   └── node/            # Full node, RPC, CLI
├── docs/                # Documentation
├── tests/               # Integration tests
├── benches/             # Benchmarks
└── devnet/              # Devnet configuration
```

## Coding Standards

### Rust Style

Follow the [Rust Style Guide](https://doc.rust-lang.org/style-guide/):

```rust
// Good: Clear function name, proper documentation
/// Calculate the total fee for a transaction batch.
///
/// # Arguments
/// * `tx_count` - Number of transactions
/// * `base_fee` - Base fee per transaction in drops
///
/// # Returns
/// Total fee in drops
pub fn calculate_total_fee(tx_count: u32, base_fee: u64) -> u64 {
    tx_count as u64 * base_fee
}

// Bad: Unclear, missing docs
fn calc_fee(n: u32, fee: u64) -> u64 {
    n as u64 * fee
}
```

### Error Handling

Use `thiserror` for error types:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("insufficient balance: {available} < {required}")]
    InsufficientBalance { available: u64, required: u64 },

    #[error("invalid transaction format: {0}")]
    InvalidFormat(String),
}
```

Use `anyhow` for application errors:

```rust
use anyhow::{Context, Result};

fn process_transaction(tx: &Transaction) -> Result<()> {
    validate(tx).context("validation failed")?;
    apply(tx).context("apply failed")?;
    Ok(())
}
```

### Documentation

All public items must have doc comments:

```rust
/// Represents a transaction in the Callchain protocol.
///
/// Transactions are the fundamental unit of state change in the ledger.
/// Each transaction modifies the ledger state in an atomic operation.
///
/// # Examples
///
/// ```
/// use callcore_protocol::Transaction;
///
/// let tx = Transaction::new_payment(
///     "cLSKzJZg4w2dgLfwf".parse()?,
///     "cN5E7s8x9y2z3w4v5u6t".parse()?,
///     1_000_000
/// )?;
/// ```
pub struct Transaction {
    // ...
}
```

### Testing

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        let fee = calculate_total_fee(10, 10);
        assert_eq!(fee, 100);
    }

    #[test]
    fn test_insufficient_balance() {
        let result = validate_payment(&account, 1_000_000);
        assert!(matches!(result, Err(ValidationError::InsufficientBalance { .. })));
    }
}
```

#### Integration Tests

```rust
// tests/payment_flow.rs
use callcore_node::{Node, Config};

#[tokio::test]
async fn test_payment_end_to_end() {
    let node = Node::start_test().await;

    let result = node.submit_payment(
        "cLSKzJZg4w2dgLfwf",
        "cN5E7s8x9y2z3w4v5u6t",
        1_000_000
    ).await;

    assert!(result.is_ok());

    let balance = node.get_balance("cN5E7s8x9y2z3w4v5u6t").await;
    assert_eq!(balance, 1_000_000);
}
```

### Logging

Use the `tracing` crate:

```rust
use tracing::{info, debug, warn, error};

pub fn process_transaction(tx: &Transaction) -> Result<()> {
    let tx_hash = tx.hash();
    debug!(%tx_hash, "processing transaction");

    match validate(tx) {
        Ok(()) => {
            info!(%tx_hash, "transaction validated");
        }
        Err(e) => {
            warn!(%tx_hash, error = %e, "transaction validation failed");
            return Err(e);
        }
    }

    Ok(())
}
```

### Performance

- Use `&str` over `String` for borrowed data
- Use `Vec::with_capacity()` when size is known
- Avoid unnecessary clones
- Profile before optimizing

## Testing Guidelines

### Test Coverage

Aim for:
- **Unit tests**: 80%+ coverage for business logic
- **Integration tests**: All public APIs
- **Property tests**: Complex algorithms
- **Fuzz tests**: Parsing and deserialization

### Test Data

Use test fixtures:

```rust
// tests/fixtures/mod.rs
pub fn sample_transaction() -> Transaction {
    Transaction::from_json(include_str!("payment.json")).unwrap()
}

pub fn test_account() -> AccountID {
    "cLSKzJZg4w2dgLfwf".parse().unwrap()
}
```

### Mocking

Use traits for testability:

```rust
pub trait LedgerView {
    fn get_account(&self, id: &AccountID) -> Option<Account>;
}

// Production implementation
pub struct DatabaseLedger { /* ... */ }
impl LedgerView for DatabaseLedger { /* ... */ }

// Test implementation
pub struct MockLedger {
    accounts: HashMap<AccountID, Account>,
}
impl LedgerView for MockLedger {
    fn get_account(&self, id: &AccountID) -> Option<Account> {
        self.accounts.get(id).cloned()
    }
}
```

## Security

### Reporting Vulnerabilities

**Do NOT open public issues for security bugs.**

Email: security@callchain.io

Include:
- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Secure Coding

- Use `sodiumoxide` for crypto (not DIY)
- Validate all inputs
- Use constant-time comparison for secrets
- Zero out sensitive memory

```rust
use sodiumoxide::crypto::sign;

// Good: Use established library
let signature = sign::sign_detached(message, &secret_key);

// Verify with constant-time comparison
if sign::verify_detached(&signature, message, &public_key).is_ok() {
    // Valid signature
}
```

## Documentation

### Code Documentation

- Every public function, struct, and trait must be documented
- Include examples in doc comments
- Document panics and errors

### External Documentation

Update relevant docs in `/docs`:
- API changes → Update `docs/api/`
- New transactions → Update `docs/transactions/`
- Architecture changes → Update `docs/architecture/`

## CI/CD

### Pre-commit Checks

Run before committing:

```bash
#!/bin/bash
# .git/hooks/pre-commit

cargo fmt -- --check || exit 1
cargo clippy --all-targets --all-features -- -D warnings || exit 1
cargo test --all || exit 1
cargo doc --no-deps || exit 1
```

### CI Pipeline

GitHub Actions runs:
1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test --all`
4. `cargo doc`
5. `cargo build --release`

## Release Process

1. Update `CHANGELOG.md`
2. Bump version in `Cargo.toml`
3. Create git tag: `git tag v1.0.0`
4. Push tag: `git push origin v1.0.0`
5. CI builds and publishes release

## Communication

### Discord

Join our Discord for real-time discussion:
https://discord.gg/callchain

### Issues

- Use templates for bug reports and features
- Include reproduction steps for bugs
- Tag appropriately

### Code Review

All PRs require:
1. At least one maintainer approval
2. All CI checks passing
3. No merge conflicts
4. Up-to-date with `main`

Review checklist:
- [ ] Code follows style guide
- [ ] Tests included
- [ ] Documentation updated
- [ ] No security issues
- [ ] Performance acceptable

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Questions?

- Check existing documentation
- Search closed issues
- Ask in Discord
- Email: dev@callchain.io

Thank you for contributing to Call-Core!
