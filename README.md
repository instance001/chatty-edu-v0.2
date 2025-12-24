\# Chatty-EDU  

\### Local-First Learning Assistant • Rust • v0.1 Spine Build



Chatty-EDU is a modular, local-first education assistant designed to run entirely on-device.  

No cloud. No accounts. No tracking.  

Just a fast, private, extensible Rust application for students, teachers, parents, and hobbyists.



This is the \*\*v0.1 spine build\*\*, reconstructed using \*\*ChattyFactory\_v1\*\*, forming the foundation for future:



\- Homework helpers  

\- Revision tools  

\- Local knowledge engines  

\- Mini-games (ChattyBox, ChattyClysm)  

\- Teacher dashboards  

\- Offline AI-augmented study workflows  



---



\## 📦 Project Structure



chatty-edu\_v0.1/

├── src/ # Rust source code

│ ├── main.rs

│ └── chatty\_edu/ # Future modules (homework, revision, games, etc.)

│

├── config/ # Settings, templates, profiles

├── homework/ # Assigned \& completed homework storage

│ ├── assigned/

│ └── completed/

│

├── revision/ # Revision packs (Year 1 – Year 6 + custom)

│ ├── year\_1/

│ ├── year\_2/

│ ├── year\_3/

│ ├── year\_4/

│ ├── year\_5/

│ ├── year\_6/

│ └── custom/

│

├── ide/ # Simple project-based learning workspace

│ └── projects/

│

├── modules/ # Educational mini-games + extensions

│ ├── chattybox/

│ ├── chattyclysm/

│ └── reserved/

│

├── runtime/ # Future local model or runtime assets

├── logs/ # Application logs (optional)

├── LICENSE # AGPLv3 license (full text)

└── Cargo.toml # Rust project manifest





---



\## 🚀 Running Chatty-EDU



You must have Rust installed (`rustup` recommended).



```bash

cd chatty-edu\_v0.1

cargo build

cargo run



This v0.1 spine includes the initial structure and stub modules.

Future versions will expand functionality as new modules come online.

🛠️ How This Build Was Generated



This project was reconstructed using ChattyFactory\_v1, the Symbound drop-sort-build system that:



&nbsp;   Accepts raw project fragments



&nbsp;   Identifies known structure



&nbsp;   Rebuilds the canonical layout



&nbsp;   Safely quarantines unknown items



&nbsp;   Produces a clean, Git-ready output folder



This v0.1 folder is the canonical baseline for future expansion.

🧾 License — AGPLv3



Chatty-EDU is free software under the GNU Affero General Public License v3 (AGPL-3.0-or-later).

This ensures:



&nbsp;   The project remains a digital commons



&nbsp;   Any network-accessible forks must contribute improvements back



&nbsp;   Commercial entities cannot enclose the codebase



Full license text included in LICENSE.

📚 Roadmap



v0.2 — Clean stubs, runtime helpers, config loader, basic state machine

v0.3 — Homework engine + revision pack loader

v0.4 — ChattyBox mini-game + teacher dashboard

v0.5 — Local reasoning model hooks (optional offline inference)

v1.0 — Fully functional offline learning assistant

👥 Credits



Built by Instance001 (Anthony) + Symbound Collective

Generated via ChattyFactory\_v1 — the Drop-Sort-Build engine.



This is a community-first, open-knowledge project.

Contributions welcome after v0.2 release.

💙 Philosophy



Tools should empower.

Learning should be local, private, and free.

Software should help people think — not replace thinking.



Chatty-EDU exists to bring that vision into the hands of students everywhere.

