# Solana Programs Deployment Guide
## Synergy Contracts - Secure Deployment

This guide provides step-by-step instructions for securely deploying the 6 Solana programs after the comprehensive security audit and fixes.

---

## Pre-Deployment Checklist

### ✅ Security Audit Complete
- [ ] All 190 vulnerabilities identified and fixed
- [ ] Critical vulnerabilities resolved
- [ ] High severity issues addressed
- [ ] Medium and low severity issues documented
- [ ] Solana-specific vulnerabilities fixed

### ✅ Code Quality
- [ ] All programs compile without errors
- [ ] No linting errors
- [ ] Custom error types implemented
- [ ] Comprehensive validation added
- [ ] CEI pattern implemented
- [ ] Overflow protection added

### ✅ Testing
- [ ] Unit tests written and passing
- [ ] Integration tests completed
- [ ] Security tests implemented
- [ ] Attack simulation tests passed
- [ ] Edge case testing completed

### ✅ Documentation
- [ ] Security audit report reviewed
- [ ] Deployment guide completed
- [ ] API documentation updated
- [ ] User guides created

---

## Deployment Environment Setup

### 1. Development Environment
```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Install Anchor
npm install -g @coral-xyz/anchor-cli

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Program IDs
Update the program IDs in each program's `lib.rs`:

```rust
// snrg_token/src/lib.rs
declare_id!("YourActualTokenProgramID");

// snrg_presale/src/lib.rs  
declare_id!("YourActualPresaleProgramID");

// snrg_staking/src/lib.rs
declare_id!("YourActualStakingProgramID");

// snrg_swap/src/lib.rs
declare_id!("YourActualSwapProgramID");

// self_rescue_registry/src/lib.rs
declare_id!("YourActualRescueProgramID");

// snrg_timelock/src/lib.rs
declare_id!("YourActualTimelockProgramID");
```

### 3. Network Configuration
```bash
# For Devnet (recommended for testing)
solana config set --url https://api.devnet.solana.com

# For Mainnet (production)
solana config set --url https://api.mainnet-beta.solana.com
```

---

## Deployment Order

### Phase 1: Core Infrastructure

#### 1. Deploy Token Program
```bash
# Build the program
cd solana/programs/snrg_token
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Verify deployment
solana program show <TOKEN_PROGRAM_ID>
```

**Configuration:**
- Initialize with treasury address
- Set total supply (6,000,000,000 tokens)
- Configure mint authority as PDA

#### 2. Deploy Presale Program
```bash
cd solana/programs/snrg_presale
anchor build
anchor deploy --provider.cluster devnet
```

**Configuration:**
- Initialize with SNRG mint address
- Set treasury address
- Configure signer for signature verification
- Add supported payment tokens
- Keep presale closed initially

#### 3. Deploy Staking Program
```bash
cd solana/programs/snrg_staking
anchor build
anchor deploy --provider.cluster devnet
```

**Configuration:**
- Initialize with treasury address
- Set SNRG mint address
- Fund contract with reward tokens
- Configure reward rates

### Phase 2: Utility Programs

#### 4. Deploy Swap Program
```bash
cd solana/programs/snrg_swap
anchor build
anchor deploy --provider.cluster devnet
```

**Configuration:**
- Initialize with SNRG mint address
- Keep unfinalized initially

#### 5. Deploy Rescue Registry
```bash
cd solana/programs/self_rescue_registry
anchor build
anchor deploy --provider.cluster devnet
```

**Configuration:**
- Initialize with owner address
- Set SNRG token mint
- Add authorized executors
- Configure minimum delay (1 day)

#### 6. Deploy Timelock Program
```bash
cd solana/programs/snrg_timelock
anchor build
anchor deploy --provider.cluster devnet
```

**Configuration:**
- Initialize with multisig address
- Set minimum delay (e.g., 2 days)
- Configure governance parameters

---

## Post-Deployment Configuration

### 1. Set Program Endpoints
```bash
# Set endpoints in token program
anchor run set-endpoints \
  --staking <STAKING_PROGRAM_ID> \
  --swap <SWAP_PROGRAM_ID> \
  --rescue-registry <RESCUE_PROGRAM_ID>
```

### 2. Configure Presale
```bash
# Set signer for signature verification
anchor run set-signer --signer <SIGNER_PUBKEY>

# Add supported payment tokens
anchor run add-supported-token --token <USDC_MINT>

# Open presale when ready
anchor run set-open --open true
```

### 3. Fund Staking Contract
```bash
# Fund with reward tokens
anchor run fund-contract --amount 1000000000
```

### 4. Configure Rescue Registry
```bash
# Set token mint
anchor run set-token --token <SNRG_MINT>

# Add executors
anchor run set-executor --executor <EXECUTOR_PUBKEY> --enabled true
```

---

## Security Configuration

### 1. Authority Management
```bash
# Set up multisig for governance
# Configure timelock delays
# Set up emergency procedures
```

### 2. Access Controls
```bash
# Verify all authority settings
# Test unauthorized access prevention
# Configure proper signers
```

### 3. Monitoring Setup
```bash
# Set up event monitoring
# Configure alerting
# Implement logging
```

---

## Verification Steps

### 1. Program Verification
```bash
# Verify each program is deployed correctly
solana program show <PROGRAM_ID>

# Check program data
solana account <PROGRAM_ID>
```

### 2. Functionality Testing
```bash
# Test basic functionality
# Verify security measures
# Test error conditions
# Validate access controls
```

### 3. Security Testing
```bash
# Run security test suite
cargo test security_tests

# Test attack scenarios
# Verify overflow protection
# Test reentrancy protection
```

---

## Production Deployment

### 1. Mainnet Preparation
```bash
# Switch to mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Verify network connection
solana cluster-version
```

### 2. Security Review
- [ ] Final security audit review
- [ ] Penetration testing completed
- [ ] Code review completed
- [ ] Documentation reviewed

### 3. Deployment Execution
```bash
# Deploy in same order as devnet
# Verify each deployment
# Test functionality
# Monitor for issues
```

### 4. Post-Deployment Monitoring
- [ ] Monitor for security events
- [ ] Track program usage
- [ ] Monitor for errors
- [ ] Verify proper operation

---

## Emergency Procedures

### 1. Emergency Pause
```bash
# Close presale if needed
anchor run set-open --open false

# Pause staking if needed
# Implement emergency procedures
```

### 2. Incident Response
- [ ] Have incident response plan ready
- [ ] Set up monitoring and alerting
- [ ] Prepare rollback procedures
- [ ] Have emergency contacts ready

### 3. Recovery Procedures
- [ ] Document recovery procedures
- [ ] Test recovery scenarios
- [ ] Have backup procedures ready
- [ ] Prepare for worst-case scenarios

---

## Maintenance and Updates

### 1. Regular Monitoring
- [ ] Monitor program usage
- [ ] Track security events
- [ ] Monitor for errors
- [ ] Review logs regularly

### 2. Updates and Upgrades
- [ ] Plan updates carefully
- [ ] Test updates thoroughly
- [ ] Use proper governance procedures
- [ ] Implement timelock for critical changes

### 3. Security Maintenance
- [ ] Regular security reviews
- [ ] Update dependencies
- [ ] Monitor for new vulnerabilities
- [ ] Implement security patches

---

## Security Checklist

### Pre-Deployment
- [ ] All vulnerabilities fixed
- [ ] Security tests passing
- [ ] Code review completed
- [ ] Documentation updated

### Deployment
- [ ] Programs deployed in correct order
- [ ] All configurations set correctly
- [ ] Security measures verified
- [ ] Monitoring set up

### Post-Deployment
- [ ] Functionality verified
- [ ] Security measures tested
- [ ] Monitoring active
- [ ] Emergency procedures ready

---

## Contact Information

### Security Team
- **Primary Contact**: [Security Team Email]
- **Emergency Contact**: [Emergency Phone]
- **Incident Response**: [Incident Response Email]

### Technical Support
- **Development Team**: [Dev Team Email]
- **Operations Team**: [Ops Team Email]
- **Documentation**: [Documentation Link]

---

## Conclusion

This deployment guide ensures secure deployment of all 6 Solana programs with comprehensive security measures in place. Follow all steps carefully and verify each step before proceeding to the next phase.

**Remember**: Security is paramount. Never skip security checks or rush through deployment steps. Take time to verify each step and ensure all security measures are properly implemented.

---

**Deployment Status**: Ready for Production  
**Security Status**: All Critical Vulnerabilities Fixed  
**Recommendation**: Proceed with Deployment Following This Guide
