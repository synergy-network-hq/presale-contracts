# BSC Contract Verification Guide

## 📁 JSON Files Generated

All Standard JSON-Input files have been created in:
```
/Users/devpup/Desktop/SynergyNetworkTestnet/synergy-contracts/ethereum/bsc-verification-json/
```

**Files:**
- `SNRGToken_standard_input.json`
- `SelfRescueRegistry_standard_input.json`
- `SNRGStaking_standard_input.json`
- `SNRGSwap_standard_input.json`
- `SNRGPresale_standard_input.json`

---

## 🎯 Step-by-Step Verification Process

### For Each Contract:

#### 1. **SNRGToken**
   - **Address:** `0xeb76FaAdff3CA1E11B33A3F6d66d8BfC16347e59`
   - **JSON File:** `SNRGToken_standard_input.json`
   - **Contract Name to Enter:** `SNRGToken`
   - **Contract Path:** `contracts/SNRGtoken.sol:SNRGToken`

#### 2. **SelfRescueRegistry**
   - **Address:** `0xde6b9F71E2ad169ee8f4b4f12341E8af047b1951`
   - **JSON File:** `SelfRescueRegistry_standard_input.json`
   - **Contract Name to Enter:** `SelfRescueRegistry`
   - **Contract Path:** `contracts/SelfRescueRegistry.sol:SelfRescueRegistry`

#### 3. **SNRGStaking**
   - **Address:** `0x9A4fEBcc26C63a91C0D928f12A41d236c52a8409`
   - **JSON File:** `SNRGStaking_standard_input.json`
   - **Contract Name to Enter:** `SNRGStaking`
   - **Contract Path:** `contracts/SNRGstaking.sol:SNRGStaking`

#### 4. **SNRGSwap**
   - **Address:** `0xEBa4d79C4Ec3aF63dd699Bf9C11767c54298d4DA`
   - **JSON File:** `SNRGSwap_standard_input.json`
   - **Contract Name to Enter:** `SNRGSwap`
   - **Contract Path:** `contracts/SNRGswap.sol:SNRGSwap`

#### 5. **SNRGPresale**
   - **Address:** `0x226Fc0bAe20Ad79f3701037E33144adF44FF43Ea`
   - **JSON File:** `SNRGPresale_standard_input.json`
   - **Contract Name to Enter:** `SNRGPresale`
   - **Contract Path:** `contracts/SNRGpresale.sol:SNRGPresale`

---

## 📝 Verification Steps on BscScan

### Step 1: Navigate to BscScan Verify Page
Go to: https://bscscan.com/verifyContract

### Step 2: Enter Contract Address
Copy and paste one of the contract addresses from above.

### Step 3: Select Verification Method
- Choose: **"Solidity (Standard-Json-Input)"**

### Step 4: Select Compiler Version
- Choose: **`v0.8.26+commit.8a97fa7a`**

### Step 5: Select License Type
- Choose: **"MIT License (MIT)"**

### Step 6: Upload JSON File
- Click **"Choose File"** or **"Upload"**
- Navigate to: `/Users/devpup/Desktop/SynergyNetworkTestnet/synergy-contracts/ethereum/bsc-verification-json/`
- Select the corresponding `*_standard_input.json` file for that contract

### Step 7: Enter Contract Name
- In the "Contract Name" field, enter the exact contract name (case-sensitive):
  - For SNRGToken: `SNRGToken`
  - For SelfRescueRegistry: `SelfRescueRegistry`
  - For SNRGStaking: `SNRGStaking`
  - For SNRGSwap: `SNRGSwap`
  - For SNRGPresale: `SNRGPresale`

### Step 8: Constructor Arguments (Optional)
- **BscScan should auto-detect these** from the blockchain
- If it doesn't, you can manually enter them (they're listed below)

### Step 9: Complete CAPTCHA
- Complete the reCAPTCHA or other verification

### Step 10: Submit
- Click "Verify and Publish"
- Wait for verification (usually takes 30 seconds to 2 minutes)

---

## 🔧 Constructor Arguments (If Manual Entry Needed)

### SNRGToken
```
0xc0f15e56f71ea8a77c6317ac8975f3bda9348872
```

### SelfRescueRegistry
```
0xc9886cD8f3c132086CbD7175898a53944E520D8a
```

### SNRGStaking
```
0xc0f15e56f71ea8a77c6317ac8975f3bda9348872
0xc9886cD8f3c132086CbD7175898a53944E520D8a
```

### SNRGSwap
```
0xeb76FaAdff3CA1E11B33A3F6d66d8BfC16347e59
0xc9886cD8f3c132086CbD7175898a53944E520D8a
```

### SNRGPresale
```
0xeb76FaAdff3CA1E11B33A3F6d66d8BfC16347e59
0xc0f15e56f71ea8a77c6317ac8975f3bda9348872
0xc9886cD8f3c132086CbD7175898a53944E520D8a
0xc9886cD8f3c132086CbD7175898a53944E520D8a
```

---

## ✅ Why Standard JSON-Input Works Best

1. **Preserves ASCII Art** - All formatting including spaces and line breaks are maintained
2. **Includes All Sources** - OpenZeppelin dependencies are included automatically
3. **Exact Compiler Settings** - Uses the exact same settings from your build
4. **Single File Upload** - No need to manually manage multiple files
5. **Auto-Detection** - Constructor arguments are usually auto-detected

---

## 🎨 Verification Success

Once verified, your contracts will display:
- ✅ Beautiful ASCII art at the top
- ✅ Complete source code with all dependencies
- ✅ Green checkmark on BscScan
- ✅ "Read Contract" and "Write Contract" tabs enabled

---

## 🆘 Troubleshooting

### Issue: "Contract Name not found"
- **Solution:** Make sure you're entering the exact contract name (case-sensitive)
- Try the full path: `contracts/SNRGtoken.sol:SNRGToken`

### Issue: "Bytecode does not match"
- **Solution:** The JSON file includes the exact compiler settings used, so this should not happen
- If it does, double-check you're using the correct JSON file for that contract address

### Issue: "Constructor arguments invalid"
- **Solution:** Let BscScan auto-detect them first
- If that fails, copy them from the "Constructor Arguments" section above

---

## 📊 Verification Checklist

- [ ] SNRGToken (`0xeb76...e59`)
- [ ] SelfRescueRegistry (`0xde6b...1951`)
- [ ] SNRGStaking (`0x9A4f...8409`)
- [ ] SNRGSwap (`0xEBa4...d4DA`)
- [ ] SNRGPresale (`0x226F...43Ea`)

---

**Good luck with your verifications! 🚀**

