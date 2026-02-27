# Signing & Distribution

## Why signing matters

Unsigned Windows executables and DLLs trigger SmartScreen warnings, antivirus false positives, and enterprise group policy blocks. Signing your binaries with an Authenticode certificate establishes publisher identity and lets Windows build reputation for your application.

winpane ships as a Rust crate (compiled into your binary), a C DLL, a Node.js native addon, or a standalone CLI host. In all cases, the final binaries you distribute need to be signed by you, the application developer.

## SmartScreen

Windows SmartScreen checks downloaded executables against a reputation database. New, unsigned binaries show a "Windows protected your PC" warning. Signed binaries from a new publisher may still show warnings initially.

Reputation builds over time as more users run your signed application without issues. EV (Extended Validation) code signing certificates bypass SmartScreen immediately because the publisher identity is verified at a higher level. Standard OV (Organization Validation) certificates require a reputation buildup period.

## Signing with Advanced Installer (MSI)

1. **Obtain a code signing certificate** from a CA (DigiCert, Sectigo, GlobalSign). EV certificates are stored on hardware tokens (USB). OV certificates can be file-based (PFX).

2. **Sign the binaries before packaging.** Use `signtool.exe` from the Windows SDK:
   ```
   signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f cert.pfx /p password winpane-host.exe
   signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f cert.pfx /p password winpane_ffi.dll
   ```

3. **Configure Advanced Installer** to sign the MSI package itself:
   - Project > Digital Signature > enable "Enable signing"
   - Select the PFX file or certificate store entry
   - Set the timestamp server URL (e.g., `http://timestamp.digicert.com`)
   - Set hash algorithm to SHA-256

4. **Build the MSI.** Advanced Installer signs the installer and all configured files during the build process.

5. **Verify signatures:**
   ```
   signtool verify /pa /v winpane-host.exe
   signtool verify /pa /v installer.msi
   ```

Always use a timestamp server. Without timestamps, signatures become invalid when the certificate expires.

## MSIX for Microsoft Store

1. **Create an MSIX package** using the MSIX Packaging Tool or by authoring an `AppxManifest.xml` manually.

2. **Register your app** in the Microsoft Partner Center. You get a publisher identity and a Store signing certificate.

3. **Package your binaries** (the winpane-host executable, any DLLs, and your application) into the MSIX layout with the manifest.

4. **Sign with your Store certificate:**
   ```
   signtool sign /fd SHA256 /a /f StoreCert.pfx /p password package.msix
   ```

5. **Submit to the Store** through Partner Center. Microsoft performs additional validation and distributes the signed package.

MSIX packages are trusted by Windows and bypass SmartScreen entirely.

## Defender allowlist

If Windows Defender flags your binary as a false positive:

1. Go to the [Microsoft Security Intelligence submission portal](https://www.microsoft.com/en-us/wdsi/filesubmission).
2. Select "Software developer" and submit the flagged file.
3. Provide details: what the software does, that it includes DirectComposition overlay rendering, and a link to your source or distribution page.
4. Response time is typically 1-5 business days.

Signing your binaries significantly reduces false positive rates. If you have an EV certificate, false positives are rare.

## For SDK consumers

If you're building an application that bundles winpane:

1. **Sign all binaries** in your distribution, including `winpane_ffi.dll` or `winpane-host.exe` if you ship them alongside your app.
2. **Don't ship debug builds.** Debug binaries include symbols and assertions that can trigger heuristic AV detections.
3. **Use a consistent certificate** across all your binaries and installers so Windows builds a single reputation profile.
4. **Test on a clean VM** before release. Run Windows Defender, submit to VirusTotal, and verify SmartScreen behavior with a fresh download.
5. **Consider MSIX or MSI packaging** rather than distributing loose executables. Installer formats are more trusted by Windows than standalone EXEs.
