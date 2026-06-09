<script setup lang="ts">
const lessonSteps = [
  {
    number: "1",
    title: "Select Skill",
    body: "Choose the review mode and lock the lesson objective.",
    color: "var(--yellow)",
  },
  {
    number: "2",
    title: "Agent Draft",
    body: "Generate a compact script, outline, and checkpoints.",
    color: "var(--blue)",
  },
  {
    number: "3",
    title: "Publish Card",
    body: "Turn the final output into a class-ready activity.",
    color: "var(--green)",
  },
];

const auditCards = [
  { label: "Audit", color: "var(--blue)", path: "search" },
  { label: "Quality", color: "var(--yellow)", path: "target" },
  { label: "Workflow", color: "var(--green)", path: "flow" },
  { label: "Standard", color: "var(--red)", path: "check" },
];
</script>

<template>
  <main class="paper-shell" aria-labelledby="page-title">
    <section class="hero">
      <div class="hero-copy">
        <p class="eyebrow">Stream Learn / Demo Board</p>
        <h1 id="page-title">
          Lesson Agent
          <span>Review Kit</span>
        </h1>
        <p class="intro">
          A warm notebook-style workspace for reviewing lesson agents, output
          quality, workflow proof, and publishing standards.
        </p>

        <div class="hero-actions" aria-label="Demo actions">
          <button class="paper-button primary" type="button">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12h14" />
              <path d="m13 6 6 6-6 6" />
            </svg>
            Start Review
          </button>
          <button class="paper-button secondary" type="button">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 6h13" />
              <path d="M8 12h13" />
              <path d="M8 18h13" />
              <path d="M3 6h.01" />
              <path d="M3 12h.01" />
              <path d="M3 18h.01" />
            </svg>
            View Queue
          </button>
        </div>
      </div>

      <div class="demo-card" aria-label="Agent output preview">
        <span class="step-badge">1</span>
        <div class="card-heading">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <rect x="6" y="8" width="12" height="10" rx="2" />
            <path d="M9 8V5h6v3" />
            <path d="M9 13h.01" />
            <path d="M15 13h.01" />
            <path d="M4 13H2" />
            <path d="M22 13h-2" />
          </svg>
          <span>Agent Output</span>
        </div>

        <div class="output-capsule">
          <svg class="code-mark" viewBox="0 0 24 24" aria-hidden="true">
            <path d="m9 18-6-6 6-6" />
            <path d="m15 6 6 6-6 6" />
          </svg>
          <div class="code-lines" aria-hidden="true">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>

        <div class="status-strip">
          <span>Two-pass audit</span>
          <strong>Ready</strong>
        </div>
      </div>
    </section>

    <section class="superpowers" aria-labelledby="superpowers-title">
      <div class="shield-mark" aria-hidden="true">
        <svg viewBox="0 0 64 64">
          <path d="M32 8 50 16v14c0 14-8.5 23-18 27-9.5-4-18-13-18-27V16Z" />
        </svg>
        <span></span>
      </div>
      <h2 id="superpowers-title" class="paper-title">SUPERPOWERS</h2>

      <div class="tile-row" aria-label="Review categories">
        <article
          v-for="card in auditCards"
          :key="card.label"
          class="paper-tile"
          :style="{ '--accent': card.color }"
        >
          <svg v-if="card.path === 'search'" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-4-4" />
          </svg>
          <svg v-else-if="card.path === 'target'" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="8" />
            <circle cx="12" cy="12" r="4" />
            <circle cx="12" cy="12" r="1" />
          </svg>
          <svg v-else-if="card.path === 'flow'" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="6" cy="7" r="3" />
            <circle cx="18" cy="17" r="3" />
            <path d="M8.5 9.5c3 4 4 5 6.5 6" />
            <path d="M6 10v7h9" />
          </svg>
          <svg v-else viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="9" />
            <path d="m8 12 3 3 6-7" />
          </svg>
          <span>{{ card.label }}</span>
        </article>
      </div>
    </section>

    <section class="lesson-flow" aria-labelledby="flow-title">
      <div class="section-heading">
        <p class="eyebrow">Review Path</p>
        <h2 id="flow-title">From rough prompt to class-ready lesson</h2>
      </div>

      <div class="flow-grid">
        <article
          v-for="step in lessonSteps"
          :key="step.number"
          class="flow-card"
          :style="{ '--badge-color': step.color }"
        >
          <span class="mini-badge">{{ step.number }}</span>
          <h3>{{ step.title }}</h3>
          <p>{{ step.body }}</p>
        </article>
      </div>
    </section>
  </main>
</template>

<style>
:root {
  --paper: #f8f1df;
  --paper-soft: #fbf7ea;
  --surface: #fffdf5;
  --ink: #334454;
  --ink-dark: #263747;
  --muted: #697174;
  --blue: #65b9e6;
  --red: #ef5549;
  --yellow: #f3b642;
  --green: #4e8b6f;
  --shadow: #2f4152;
  --line-soft: rgba(51, 68, 84, 0.08);
  color: var(--ink-dark);
  background: var(--paper);
  font-family: Nunito, Avenir, "Segoe UI", sans-serif;
  font-size: 16px;
  font-synthesis: none;
  line-height: 1.5;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

* {
  box-sizing: border-box;
}

body {
  min-width: 320px;
  min-height: 100vh;
  margin: 0;
  background-color: var(--paper);
}

button {
  font: inherit;
}
</style>

<style scoped>
.paper-shell {
  min-height: 100vh;
  padding: 36px;
  overflow: hidden;
  background-color: var(--paper);
  background-image:
    linear-gradient(rgba(51, 68, 84, 0.045) 1px, transparent 1px),
    linear-gradient(90deg, rgba(51, 68, 84, 0.045) 1px, transparent 1px);
  background-size: 24px 24px;
}

.hero {
  width: min(1180px, 100%);
  min-height: calc(100vh - 72px);
  margin: 0 auto;
  display: grid;
  grid-template-columns: minmax(0, 0.92fr) minmax(420px, 1.08fr);
  align-items: center;
  gap: 54px;
}

.hero-copy {
  position: relative;
  z-index: 1;
}

.eyebrow {
  margin: 0 0 12px;
  color: var(--green);
  font-size: 0.82rem;
  font-weight: 900;
  letter-spacing: 0;
}

h1,
h2,
h3,
p {
  overflow-wrap: anywhere;
}

h1 {
  max-width: 650px;
  margin: 0;
  color: var(--ink-dark);
  font-family: Fraunces, "Libre Baskerville", Georgia, serif;
  font-size: clamp(3.2rem, 8vw, 7.6rem);
  line-height: 0.88;
  letter-spacing: 0;
}

h1 span {
  display: block;
  color: var(--red);
  text-shadow: 5px 5px 0 var(--ink);
}

.intro {
  max-width: 580px;
  margin: 28px 0 0;
  color: var(--ink);
  font-size: clamp(1rem, 1.4vw, 1.2rem);
  font-weight: 700;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-top: 34px;
}

.paper-button {
  min-height: 54px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  border: 4px solid var(--ink);
  border-radius: 16px;
  color: var(--ink-dark);
  cursor: pointer;
  font-weight: 900;
  transition:
    transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.paper-button svg {
  width: 22px;
  height: 22px;
  fill: none;
  stroke: currentColor;
  stroke-width: 3;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.paper-button.primary {
  padding: 0 22px;
  background: var(--yellow);
  box-shadow: 7px 7px 0 var(--ink);
}

.paper-button.secondary {
  padding: 0 20px;
  background: var(--surface);
  box-shadow: 7px 7px 0 rgba(51, 68, 84, 0.22);
}

.paper-button:hover {
  transform: translate(-2px, -2px);
  box-shadow: 9px 9px 0 var(--ink);
}

.paper-button:active {
  transform: translate(2px, 2px);
  box-shadow: 4px 4px 0 var(--ink);
}

.paper-button:focus-visible,
.paper-tile:focus-visible {
  outline: 4px solid var(--blue);
  outline-offset: 4px;
}

.demo-card {
  position: relative;
  min-height: 456px;
  padding: 74px clamp(26px, 4vw, 56px) 44px;
  border: 4px solid var(--ink);
  border-radius: 20px;
  background: rgba(255, 253, 245, 0.94);
  box-shadow: 18px 18px 0 rgba(47, 65, 82, 0.16);
}

.step-badge {
  position: absolute;
  top: -24px;
  left: -24px;
  width: 52px;
  height: 52px;
  display: grid;
  place-items: center;
  border: 4px solid var(--ink);
  border-radius: 50%;
  background: var(--yellow);
  color: var(--ink-dark);
  font-family: Georgia, serif;
  font-size: 1.35rem;
  font-weight: 900;
}

.card-heading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 18px;
  color: var(--ink-dark);
  font-family: Georgia, serif;
  font-size: clamp(1.8rem, 3.4vw, 3rem);
  font-weight: 700;
}

.card-heading svg {
  width: 54px;
  height: 54px;
  fill: none;
  stroke: var(--blue);
  stroke-width: 3.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.output-capsule {
  width: min(760px, 100%);
  min-height: 138px;
  margin: 64px auto 0;
  padding: 30px 34px;
  display: grid;
  grid-template-columns: 72px 1fr;
  align-items: center;
  gap: 24px;
  border-radius: 18px;
  background: #344b60;
  box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.04);
}

.code-mark {
  width: 62px;
  height: 62px;
  fill: none;
  stroke: var(--blue);
  stroke-width: 3;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.code-lines {
  display: grid;
  gap: 16px;
}

.code-lines span {
  display: block;
  height: 14px;
  border-radius: 999px;
  background: rgba(159, 190, 211, 0.46);
}

.code-lines span:nth-child(1) {
  width: 70%;
}

.code-lines span:nth-child(2) {
  width: 92%;
}

.code-lines span:nth-child(3) {
  width: 54%;
}

.status-strip {
  width: min(760px, 100%);
  margin: 30px auto 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  color: var(--muted);
  font-weight: 800;
}

.status-strip strong {
  display: inline-flex;
  min-width: 78px;
  justify-content: center;
  padding: 6px 12px;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--green);
  color: var(--surface);
}

.superpowers {
  width: min(880px, 100%);
  margin: -48px auto 96px;
  text-align: center;
}

.shield-mark {
  position: relative;
  width: 170px;
  height: 170px;
  margin: 0 auto 22px;
  display: grid;
  place-items: center;
  border: 5px solid var(--ink);
  border-radius: 40px;
  background: var(--red);
  box-shadow: 16px 16px 0 var(--ink);
}

.shield-mark svg {
  width: 88px;
  height: 88px;
  fill: none;
  stroke: var(--surface);
  stroke-width: 5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.shield-mark span {
  position: absolute;
  top: -23px;
  right: -18px;
  width: 48px;
  height: 52px;
  background: var(--yellow);
  clip-path: polygon(46% 0, 100% 0, 67% 38%, 100% 38%, 28% 100%, 45% 53%, 0 53%);
  filter: drop-shadow(3px 3px 0 var(--ink));
}

.paper-title {
  margin: 0 0 54px;
  color: var(--red);
  font-family: Fraunces, "Libre Baskerville", Georgia, serif;
  font-size: clamp(2.5rem, 6vw, 5.6rem);
  line-height: 0.95;
  letter-spacing: 0;
  text-shadow: 4px 4px 0 var(--ink);
}

.tile-row {
  display: grid;
  grid-template-columns: repeat(4, minmax(112px, 1fr));
  gap: 26px;
}

.paper-tile {
  min-height: 142px;
  padding: 20px 12px 16px;
  display: grid;
  place-items: center;
  gap: 12px;
  border: 4px solid var(--ink);
  border-radius: 20px;
  background: var(--surface);
  color: var(--ink-dark);
  box-shadow: 9px 9px 0 var(--ink);
  transition:
    transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.paper-tile:hover {
  transform: translate(-2px, -2px);
  box-shadow: 11px 11px 0 var(--ink);
}

.paper-tile svg {
  width: 54px;
  height: 54px;
  fill: none;
  stroke: var(--accent);
  stroke-width: 3.3;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.paper-tile span {
  min-height: 24px;
  color: var(--ink-dark);
  font-family: Georgia, serif;
  font-size: 1.05rem;
  font-weight: 700;
}

.lesson-flow {
  width: min(1100px, 100%);
  margin: 0 auto 42px;
}

.section-heading {
  max-width: 680px;
  margin-bottom: 28px;
}

.section-heading h2 {
  margin: 0;
  color: var(--ink-dark);
  font-family: Fraunces, Georgia, serif;
  font-size: clamp(2rem, 4vw, 4rem);
  line-height: 1;
  letter-spacing: 0;
}

.flow-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 22px;
}

.flow-card {
  position: relative;
  min-height: 220px;
  padding: 52px 24px 26px;
  border: 4px solid var(--ink);
  border-radius: 18px;
  background: var(--surface);
  box-shadow: 12px 12px 0 rgba(47, 65, 82, 0.18);
}

.mini-badge {
  position: absolute;
  top: -20px;
  left: 22px;
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  border: 4px solid var(--ink);
  border-radius: 50%;
  background: var(--badge-color);
  color: var(--ink-dark);
  font-weight: 900;
}

.flow-card h3 {
  margin: 0 0 12px;
  color: var(--ink-dark);
  font-family: Georgia, serif;
  font-size: 1.45rem;
  line-height: 1.1;
}

.flow-card p {
  margin: 0;
  color: var(--muted);
  font-weight: 800;
}

@media (max-width: 920px) {
  .paper-shell {
    padding: 26px;
  }

  .hero {
    min-height: auto;
    grid-template-columns: 1fr;
    gap: 44px;
    padding: 40px 0 84px;
  }

  .demo-card {
    min-height: 390px;
  }

  .superpowers {
    margin-top: 0;
  }

  .tile-row {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .flow-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 560px) {
  .paper-shell {
    padding: 18px;
    background-size: 20px 20px;
  }

  h1 {
    font-size: clamp(3rem, 18vw, 4.6rem);
  }

  .hero-actions {
    display: grid;
  }

  .paper-button {
    width: 100%;
  }

  .demo-card {
    min-height: 330px;
    padding: 54px 18px 28px;
    border-width: 3px;
    box-shadow: 10px 10px 0 rgba(47, 65, 82, 0.16);
  }

  .step-badge {
    top: -18px;
    left: -12px;
    width: 42px;
    height: 42px;
    border-width: 3px;
  }

  .card-heading {
    gap: 10px;
    font-size: 1.52rem;
  }

  .card-heading svg {
    width: 40px;
    height: 40px;
  }

  .output-capsule {
    min-height: 118px;
    grid-template-columns: 42px 1fr;
    gap: 14px;
    margin-top: 42px;
    padding: 22px 18px;
  }

  .code-mark {
    width: 42px;
    height: 42px;
  }

  .status-strip {
    align-items: flex-start;
    flex-direction: column;
  }

  .shield-mark {
    width: 136px;
    height: 136px;
    border-radius: 32px;
    box-shadow: 12px 12px 0 var(--ink);
  }

  .paper-title {
    margin-bottom: 34px;
  }

  .tile-row {
    gap: 16px;
  }

  .paper-tile {
    min-height: 126px;
    border-width: 3px;
    box-shadow: 7px 7px 0 var(--ink);
  }

  .paper-tile svg {
    width: 44px;
    height: 44px;
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
</style>
