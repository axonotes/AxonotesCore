<p align="center">
  <img src="https://raw.githubusercontent.com/axonotes/.github/refs/heads/main/logo_no_text.png" alt="Axonotes Logo" width="150"/>
</p>

<h1 align="center">AxonotesCore 🐙</h1>

<p align="center">
  <strong>Core monorepo for the Axonotes Desktop application (Tauri/SvelteKit) and its SpaceTimeDB Rust backend.</strong>
  <br />
  <em>Currently in early planning and development.</em>
</p>

<p align="center">
  <a href="#about-axonotes">About Axonotes</a> •
  <a href="#whats-inside-this-monorepo">What's Inside?</a> •
  <a href="#current-stage">Current Stage</a> •
  <a href="#tech-stack">Tech Stack</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#stay-connected">Stay Connected</a> •
  <a href="#license">License</a>
</p>

[![Status](https://img.shields.io/badge/status-early%20development-orange)](https://github.com/axonotes/AxonotesCore) <!-- Replace with actual repo link -->

---

## 🎯 About Axonotes

Axonotes is envisioned as the ultimate command center for students and educators, designed to end the chaos of juggling
multiple applications. Our goal is to create a single, streamlined platform where notes, collaboration, planning,
learning tools (like flashcards and interactive exercises), and communication live together seamlessly.

We're building an all-in-one academic suite focused on:

* **Unified Workflow:** Notes, tasks, chat, and learning tools in one place.
* **Effortless Collaboration:** Real-time co-authoring with features like line-level locking.
* **Powerful Knowledge Creation:** Flexible note-taking (Markdown, rich text, infinite canvases, pen support), \(
  \LaTeX \) support, and more.
* **Smart Organization:** Integrated planning, powerful global search, and knowledge graphs.
* **Offline-First & Cross-Platform:** Work anywhere, anytime, on any device.
* **Revolutionary Version History:** Every change saved, powered by SpaceTimeDB.
* **Secure & Private by Design:** Built with Swiss precision.

## 📦 What's Inside This Monorepo?

This `AxonotesCore` repository is a monorepo that houses the foundational code for Axonotes:

* **`/app`**:
  * The Axonotes desktop application.
  * Built with **[Tauri](https://tauri.app/)** (using **[SvelteKit](https://kit.svelte.dev/)** for the frontend).
  * Provides the cross-platform user interface (Windows, macOS, Linux) and client-side logic.
  * Handles offline-first capabilities and synchronization with the backend.

* **`/server`**:
  * The backend logic and data modules running on **[SpaceTimeDB](https://spacetimedb.com/)**.
  * Written in **Rust**.
  * Manages real-time collaboration, data persistence, and the detailed version history system.

*(Note: Directory names are placeholders and may evolve.)*

## ⏳ Current Stage

Axonotes and this `AxonotesCore` repository are currently in the **early planning and development phase**. The code here
represents foundational work and is subject to significant changes as we iterate and refine our vision based on
community feedback.

## 🛠️ Tech Stack

* **Client-side (Desktop App):**
  * Framework: [Tauri](https://tauri.app/)
  * UI: [SvelteKit](https://kit.svelte.dev/)
  * Language: TypeScript, HTML, CSS
* **Backend & Real-time Database:**
  * Platform: [SpaceTimeDB](https://spacetimedb.com/)
  * Language: Rust
* **Key Features Powered by this Stack:**
  * Cross-platform native-like experience
  * Real-time collaboration
  * Robust offline-first capabilities
  * Incredibly detailed version history

## 🚀 Getting Started

As we are in the early stages, detailed setup and contribution guidelines for developers are still being formulated.

However, to work with this repository, you will generally need:

* **Rust Toolchain:** For the SpaceTimeDB modules.
* **Node.js & bun:** For the SvelteKit frontend and Tauri.
* **Tauri Prerequisites:** Follow the [Tauri setup guide](https://tauri.app/v1/guides/getting-started/prerequisites) for
  your operating system.

More specific instructions for building, running, and developing will be added to the respective subdirectories (`/app`,
`/server`) as they mature.

## 🤝 Contributing

Your insights, experiences, and ideas are critical at this early stage! While direct code contributions to
`AxonotesCore` will become more streamlined as the project matures, here's how you can help shape Axonotes right now:

* 📧 **Share Your Thoughts via Email:** Send your ideas, your biggest frustrations with current tools, and your dream features to:
  `oliver@axonotes.ch`
* 📝 **Fill Out Our Quick Survey:** [https://forms.gle/N2qFoXn4PonD6EnA9](https://forms.gle/N2qFoXn4PonD6EnA9)
* ⭐ **Watch this Repository:** Stay updated on our progress.
* 💡 **Open Issues:** Feel free to open issues in this repository for specific bugs you anticipate or features directly
  related to the core application's structure or functionality.
* 🗣️ **Spread the Word:** Sharing Axonotes with friends, classmates, and colleagues helps immensely!

We plan to be open to direct Pull Request suggestions for features and improvements that may be accepted into the core
product. Formal contribution guidelines (`CONTRIBUTING.md`) will be added as the codebase stabilizes.

## 🌐 Stay Connected with Axonotes

Follow the overall Axonotes project for updates, announcements, and community discussion:

* **Website:** [axonotes.ch](https://axonotes.ch) (Coming Soon!)
* **X (Twitter):** [@axonotes](https://twitter.com/axonotes)
* **YouTube:** [@axonotes](https://youtube.com/@axonotes)
* **Instagram:** [@axonotes](https://instagram.com/axonotes)
* **BlueSky:** [@axonotes.bsky.social](https://bsky.app/profile/axonotes.bsky.social)
* **TikTok:** [@axonotes_ch](https://www.tiktok.com/@axonotes_ch)

## 📜 License

**AxonotesCore is licensed under the Business Source License 1.1 (BSL 1.1).**

**Please note:** The following is a brief summary and not legal advice. You should always refer to the
full [LICENSE](LICENSE) file in this repository for the complete terms and conditions.

**Key Points of the BSL 1.1 for AxonotesCore (Version 0.0.0):**

* **Before the Change Date (2030-05-02):**
  * **You CAN:**
    * Copy, modify, create derivative works, and redistribute the Licensed Work.
    * Make non-production use of the Licensed Work.
    * Make limited production use under the "Additional Use Grant":
      * Self-host for your own or your organization's internal purposes.
      * The self-hosted deployment must serve **no more than 50 (fifty) individual users**.
      * You **cannot** offer the Licensed Work to third parties as a distinct, stand-alone hosted or managed
        service for payment or other commercial benefit primarily derived from providing access to the Licensed
        Work.
  * **You CANNOT (without a separate commercial license from Axonotes):**
    * Use the Licensed Work in production beyond the conditions of the "Additional Use Grant" (e.g., self-hosting
      for more than 50 users or offering it as a commercial hosted service).
* **On or After the Change Date (2030-05-02):**
  * The license for AxonotesCore (Version 0.0.0) will automatically convert to the **GNU Affero General Public License
    v3.0 or later (AGPLv3+)**.
  * The terms of the BSL 1.1 will terminate, and the AGPLv3+ terms will apply.
* **Copyright:** (c) 2025 Axonotes (Licensor: Oliver Seifert)
* **Important:**
  * You must conspicuously display the BSL 1.1 license on each original or modified copy of the Licensed Work.
  * Any use in violation of the BSL 1.1 will automatically terminate your rights under the license.
* **Additional Permissions for AGPLv3 (effective after Change Date):**
  * The license includes additional permissions under GNU GPL version 3 section 7 and GNU AGPL version 3 section 13
    concerning linking or combining with SpaceTimeDB (which is covered by AGPL v3.0). These permissions address
    certain obligations when conveying the combined work, particularly regarding the offering of Corresponding Source
    for network interactions. Please see the full license text for details.

**This license is not an open-source license until the Change Date.**

For the full terms, conditions, and definitions, please consult the [LICENSE](LICENSE) file.

---

Thank you for your interest in AxonotesCore! We're excited to build the future of academic software with you.

Best regards,
Oliver & the (future) Axonotes Team
(A Swiss-based initiative)
