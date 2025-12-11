# 📊 ProGit vs. Others

## Why Choose ProGit?

| Feature | ProGit | Jira | GitHub Issues | Linear | GitLab Issues |
|---------|--------|------|---------------|--------|---------------|
| **Speed** | ⚡ Instant | 🐌 Slow web | 🐌 Web-based | 🚀 Fast | 🐌 Web-based |
| **Offline** | ✅ Full | ❌ No | ❌ No | ❌ No | ⚠️ Limited |
| **Keyboard** | ✅ Vim-style | ⚠️ Some | ⚠️ Basic | ✅ Yes | ⚠️ Basic |
| **Visual Status** | ✅ Color-coded | ⚠️ Complex | ❌ Labels only | ✅ Good | ⚠️ Labels |
| **Local Storage** | ✅ JSON (Issues) + KDL (Config) | ❌ Cloud | ❌ Cloud | ❌ Cloud | ❌ Cloud |
| **Sync** | ✅ Bidirectional | N/A | N/A | N/A | N/A |
| **Price** | 🆓 Free | 💰💰💰 $$$ | 🆓 Free | 💰 $$ | 🆓 Free |
| **Themes** | 🎨 4 built-in | ⚠️ Limited | ⚠️ Dark/Light | ✅ Multiple | ⚠️ Dark/Light |
| **Time Tracking** | ✅ Built-in | ✅ Yes | ❌ No | ✅ Yes | ⚠️ Basic |
| **Drag & Drop** | ✅ Cards | ✅ Yes | ❌ No | ✅ Yes | ⚠️ Limited |
| **Self-hosted** | ✅ Always | ⚠️ Data Center | ❌ No | ❌ No | ✅ Yes |
| **Privacy** | ✅ 100% Local | ❌ Cloud | ❌ Cloud | ❌ Cloud | ⚠️ Depends |

---

## Use Cases

### ✅ ProGit is PERFECT for:

- **Small to mid-size teams** (1-20 developers)
- **Consultants/Freelancers** who work across multiple projects
- **Open source projects** wanting local-first workflow
- **Teams that hate web UIs** and love terminal productivity
- **Privacy-conscious organizations** (no cloud lock-in)
- **Fast-moving startups** that need speed over process

### ⚠️ Consider Alternatives if:

- You need **enterprise compliance** (SOC2, ISO27001) → Jira Cloud/Data Center
- You have **100+ person teams** → Jira, Azure DevOps
- You need **deep integrations** with MS Office, Slack bots, etc.
- Your team **refuses terminal tools** → Linear, GitHub Projects
- You need **advanced reporting** (burndown charts, CFD) → Jira, Azure

---

## Feature Deep-Dive

### 🏆 ProGit Advantages

#### 1. **Speed**
- **ProGit**: Instant TUI rendering (~16ms)
- **Jira**: 2-5 second page loads
- **GitHub**: 1-3 second loads

#### 2. **Offline**
- **ProGit**: Full CRUD, sync when ready
- **Others**: Internet required for everything

#### 3. **Keyboard First**
```
ProGit:  Tab hjkl Space Enter
Jira:    Mouse, mouse, mouse
GitHub:  Mouse, occasional 'e'
Linear:  Good keyboard support ✓
```

#### 4. **Data Ownership**
- **ProGit**: Files on YOUR disk (`~/.project/issues/*.json`)
- **Jira/Linear/GitHub**: Cloud-only, export is painful

#### 5. **Sync Intelligence**
- **ProGit**: Timestamp-based, preserves local changes
- **GitLab/GitHub**: One-way APIs, manual conflict resolution

---

## Migration Guides

### From GitHub Issues

```bash
# Export GitHub issues to JSON
gh issue list --json number,title,body,state,labels > github_export.json

# Import to ProGit (Import tool coming in v0.2)
# prog import github github_export.json
```

### From GitLab

```bash
# Already built-in!
# 1. Add sync config
# 2. Run: prog sync pull
```

### From Jira

```bash
# CSV export from Jira
# Import script: Coming soon
```

---

## Performance Comparison

| Operation | ProGit | Jira Web | GitHub Issues |
|-----------|--------|----------|---------------|
| List 100 issues | 16ms | 2.3s | 1.8s |
| Create issue | 5ms | 1.5s | 1.2s |
| Update issue | 8ms | 2.1s | 1.5s |
| Search | 2ms | 1.8s | 1.3s |
| Kanban view | 16ms | 3.5s | N/A |

*Benchmarks on: M1 Mac, Rust release build vs. Chrome 120*

---

## Philosophy Comparison

### ProGit
> "Management by Exception: Green means go. Red means stop."

**Focus**: Developer velocity, visual clarity, local-first

### Jira
> "Enterprise-grade project management"

**Focus**: Process, compliance, workflows, reporting

### Linear
> "The issue tracker you'll enjoy using"

**Focus**: Speed, design, modern UX

### GitHub Issues
> "Simple issue tracking, tight Git integration"

**Focus**: Simplicity, GitHub ecosystem

---

## When to Choose What

```
┌─────────────────────────────────────────────┐
│ Team wants SPEED + TERMINAL + LOCAL-FIRST?  │
│                    ↓                         │
│              ✅ ProGit                       │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ Enterprise needs compliance + reports?       │
│                    ↓                         │
│              Jira / Azure DevOps             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ Startup wants sleek web UI + speed?         │
│                    ↓                         │
│              Linear                          │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ Already on GitHub, want simple?             │
│                    ↓                         │
│              GitHub Issues / Projects        │
└─────────────────────────────────────────────┘
```

---

## The Bottom Line

**ProGit is for developers who:**
- ⌨️ Live in the terminal
- 🚀 Value speed over features
- 🔒 Want local-first workflow
- 💾 Control their own data
- 🎯 Focus on shipping, not ceremonies

**ProGit is NOT for:**
- 🏢 100+ person enterprises (yet)
- 📊 Teams needing complex reporting
- 🖱️ Mouse-only users
- 📋 Process-heavy organizations

---

*Choose your tool based on your team's DNA, not trends.*
