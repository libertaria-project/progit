# The Libertaria Venture License (LVL-1.0)

### ⚡ Developer Summary (TL;DR)

**This is the "Glass Box" license.**

It allows for closed-source, proprietary distribution, but demands total cryptographic transparency regarding the build process.

We enforce **Trust through Verification**:

1.  **Closed Source:** You are allowed to distribute binaries (executables) without releasing your source code to the public. Your trade secrets remain yours.

2.  **Open Provenance:** You must provide the **Build Manifest** (cryptographic hash tree and logs) inside the package. We may not see the code, but we must be able to verify _that_ it was built cleanly from a specific state.

3.  **Verified Identity:** You must be a **Registered Entity** within the project's **Trust Registry**. Anonymous proprietary blobs are not allowed.

**Hide the Code. Prove the Build.**

---

### 🛡️ Why LVL-1.0?

-   **Enterprise Ready:** Enables hardware vendors, defense contractors, and commercial entities to integrate into open ecosystems without exposing critical IP.

-   **Supply Chain Security:** Eliminates the "Black Box" risk. Even if the code is closed, the build chain is transparent. If the cryptographic proofs fail, the package is rejected.

-   **Accountability:** Creates consequences. If an audit reveals malicious injection or hidden backdoors, the entity's cryptographic trust status and license are immediately revoked.

-   **Fortified:** Governed by **Dutch Law** (Amsterdam) for absolute _Rechtssicherheit_ regarding commercial contracts.

*For the full legal text, see the `LICENSE` file.*