# 📦 Creating OTA Bundles

This guide walks through building, signing, and publishing frontend bundles for hotswap delivery.

---

## 1. Build your frontend

```bash
pnpm build
# or: npm run build, yarn build, etc.
```

This produces your `dist/` or `build/` directory — the same output that Tauri embeds via `frontendDist`.

---

## 2. Create the archive

### tar.gz (default, no extra features needed)

```bash
tar -czf frontend.tar.gz -C dist .
```

### zip (requires `features = ["zip"]`)

```bash
cd dist && zip -r ../frontend.zip . && cd ..
```

> ⚠️ The archive root should contain `index.html` directly — not a subdirectory. Use `-C dist .` (not `-C . dist`).
>
> **Note:** `tar -C dist .` produces entries with a `./` prefix (e.g. `./index.html`). This is fine — the plugin strips leading `./` components during extraction. What is rejected: absolute paths and `..` path traversal.

---

## 3. Sign the bundle

Use the same signing key you generated for the plugin (see [Security > Signing Guide](security.md#signing-guide)).

```bash
# Using the Tauri CLI
pnpm tauri signer sign frontend.tar.gz -k ~/.tauri/hotswap.key

# Or with minisign directly
minisign -Sm frontend.tar.gz -s ~/.tauri/hotswap.key
```

This creates `frontend.tar.gz.sig`. Read its contents — this is the `signature` field in your manifest.

```bash
# Read the signature
cat frontend.tar.gz.sig
```

---

## 4. Upload to your CDN

Upload both files:

```bash
# AWS S3
aws s3 cp frontend.tar.gz s3://my-bucket/updates/v0.1.0-ota.3/
aws s3 cp frontend.tar.gz.sig s3://my-bucket/updates/v0.1.0-ota.3/

# Or any HTTPS-accessible host
```

---

## 5. Publish the manifest

Make your update endpoint return the manifest pointing to the uploaded bundle:

```json
{
  "version": "0.1.0-ota.3",
  "sequence": 43,
  "min_binary_version": "0.1.0",
  "url": "https://cdn.example.com/updates/v0.1.0-ota.3/frontend.tar.gz",
  "signature": "untrusted comment: signature from minisign secret key\nRWQ...<base64>...",
  "notes": "Fixed dark mode colors",
  "bundle_size": 2097152
}
```

See the full [Server Contract](server-contract.md) for details.

---

## CI/CD Example (GitHub Actions)

```yaml
name: OTA Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'OTA version (e.g. 0.1.0-ota.3)'
        required: true

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - run: npm ci
      - run: npm run build

      - name: Create bundle
        run: tar -czf frontend.tar.gz -C dist .

      - name: Sign bundle
        run: |
          echo "${{ secrets.HOTSWAP_PRIVATE_KEY }}" > /tmp/hotswap.key
          npx @tauri-apps/cli signer sign frontend.tar.gz -k /tmp/hotswap.key
          rm /tmp/hotswap.key

      - name: Upload to S3
        run: |
          aws s3 cp frontend.tar.gz s3://my-bucket/updates/v${{ inputs.version }}/
          aws s3 cp frontend.tar.gz.sig s3://my-bucket/updates/v${{ inputs.version }}/

      - name: Publish manifest
        run: |
          SIGNATURE=$(cat frontend.tar.gz.sig)
          SIZE=$(stat -f%z frontend.tar.gz 2>/dev/null || stat -c%s frontend.tar.gz)
          curl -X POST https://api.example.com/updates \
            -H "Authorization: Bearer ${{ secrets.API_TOKEN }}" \
            -H "Content-Type: application/json" \
            -d "{
              \"version\": \"${{ inputs.version }}\",
              \"sequence\": ${{ github.run_number }},
              \"min_binary_version\": \"0.1.0\",
              \"url\": \"https://cdn.example.com/updates/v${{ inputs.version }}/frontend.tar.gz\",
              \"signature\": $(echo "$SIGNATURE" | jq -Rs .),
              \"bundle_size\": $SIZE
            }"
```

> 💡 Using `${{ github.run_number }}` as the sequence is a simple way to get a monotonically increasing counter.

---

## Versioning Convention

| Event | Version | Sequence |
|-------|---------|----------|
| Binary release `0.1.0` | — | — |
| First OTA hotfix | `0.1.0-ota.1` | 1 |
| Second OTA hotfix | `0.1.0-ota.2` | 2 |
| Binary release `0.2.0` | — | — |
| First OTA for `0.2.0` | `0.2.0-ota.1` | 3 |

Sequences are global and always increase. The `version` field is for display only.
