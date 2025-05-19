<p align="center">
  <a href="./">
    <img src="assets/logo_no_text.png" alt="Axonotes Logo" width="150"/>
  </a>
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
  <a href="#stay-connected-with-axonotes">Stay Connected with Axonotes</a> •
  <a href="#star-history">⭐ Star History</a> •
  <a href="#license-overview">License Overview</a>
</p>

[![Status](https://img.shields.io/badge/status-early%20development-orange)](https://github.com/axonotes/AxonotesCore)

---

## 🎯 About Axonotes

Axonotes is envisioned as the ultimate command center for students and educators, designed to end the chaos of juggling
multiple applications. Our goal is to create a single, streamlined platform where notes, collaboration, planning,
learning tools (like flashcards and interactive exercises), and communication live together seamlessly.

We're building an all-in-one academic suite focused on:

* **Unified Workflow:** Notes, tasks, chat, and learning tools in one place.
* **Effortless Collaboration:** Real-time co-authoring with features like line-level locking.
* **Powerful Knowledge Creation:** Flexible note-taking (Markdown, rich text, infinite canvases, pen support), `LaTeX` 
    support, and more.
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

> **Note:** Directory names are placeholders and may evolve.

## ⏳ Current Stage

Axonotes and this `AxonotesCore` repository are currently in the **early planning and development phase**. The code here
represents foundational work and is subject to significant changes as we iterate and refine our vision based on
community feedback.

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=axonotes/AxonotesCore&type=Date)](https://www.star-history.com/#axonotes/AxonotesCore&Date)

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

*   **Website:** [axonotes.ch](https://axonotes.ch) (Coming Soon!)
*   **Discord Server:** [https://discord.gg/myBMaaDeQu](https://discord.gg/myBMaaDeQu)
*   **X (Twitter):** [@axonotes](https://twitter.com/axonotes)
*   **YouTube:** [@axonotes](https://youtube.com/@axonotes)
*   **Reddit:** [r/axonotes](https://www.reddit.com/r/Axonotes/)
*   **BlueSky:** [@axonotes.bsky.social](https://bsky.app/profile/axonotes.bsky.social)

## 📜 License Overview

AxonotesCore (Version 0.0.0) is licensed under the **Business Source License 1.1 (BSL 1.1)**.

* **Until May 2, 2030 (the "Change Date"):**
  * You **CAN** copy, modify, create derivative works, and redistribute the software.
  * You **CAN** use it for non-production purposes.
  * For **production use**, you can self-host it for internal purposes for **up to 50 individual users**.
  * You **CANNOT** offer it as a commercial hosted service or exceed the 50-user limit in production without a separate commercial license from Axonotes.
* **On or After May 2, 2030:**
  * The license will automatically convert to the **GNU Affero General Public License v3.0 or later (AGPLv3+)**.
* **Important:**
  * The software is **not considered open-source** until the Change Date.
  * You must include the BSL 1.1 license text with any distribution.

This is a brief summary. For full terms and conditions, please see the [LICENSE](LICENSE) file.

---

Thank you for your interest in AxonotesCore! We're excited to build the future of academic software with you.

Best regards,
Oliver & the (future) Axonotes Team
(A Swiss-based initiative)
