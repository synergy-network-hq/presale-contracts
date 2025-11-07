#!/bin/bash

# Script to decrypt the private key
# This will prompt you for your password

echo "Please enter your password:"
read -s PASSWORD

if [ -z "$PASSWORD" ]; then
    echo "Error: Password cannot be empty"
    exit 1
fi

# Generate key from password
echo -n "$PASSWORD" | openssl dgst -sha256 -binary > key.bin

# Decrypt the data
echo "Decrypting..."
openssl enc -aes-256-cbc -d \
  -in encrypted.bin \
  -K $(xxd -p -c 64 key.bin) \
  -iv $(xxd -p -c 32 iv.bin)

echo ""
echo "Decryption complete. The output above is your private key."

# Clean up
rm decoded.bin iv.bin encrypted.bin key.bin

echo "Temporary files cleaned up."

