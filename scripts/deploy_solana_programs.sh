#!/bin/bash

# This script builds and (optionally) deploys all of the Anchor‑based SNRG
# programs to the Solana mainnet.  It iterates through each program in the
# `solana/programs` directory, builds the program artefacts and then
# performs deployment.  You must have the Anchor CLI installed and
# configured with your Solana keypair and network.  The script assumes you
# have already created the `anchor.toml` file at the root of this project
# that defines each program and its corresponding program ID.
#
# To build and deploy, run:
#   ./scripts/deploy_solana_programs.sh
#
# The script uses `anchor build` and `anchor deploy` commands.  If you only
# wish to build without deployment, set the environment variable
# `DEPLOY=false` before executing this script.

set -euo pipefail

# Ensure we run from the repository root
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DEPLOY=${DEPLOY:-true}

echo "Building all Solana programs..."
for program_dir in solana/programs/*; do
  if [[ -d "$program_dir" ]]; then
    echo "Building $(basename "$program_dir")..."
    (cd "$program_dir" && anchor build)
  fi
done

if [[ "$DEPLOY" != "false" ]]; then
  echo "Deploying programs to Solana mainnet..."
  # The --provider.cluster option selects the cluster.  Adjust to your network
  # (e.g. mainnet, devnet, testnet) as needed.  This deploys all programs
  # declared in Anchor.toml in the root directory.
  anchor deploy --provider.cluster mainnet
  echo "Solana programs deployed successfully."
else
  echo "Skipping deployment because DEPLOY=false"
fi