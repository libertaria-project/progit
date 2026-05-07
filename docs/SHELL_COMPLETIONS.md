# Shell Completions

Enable tab completion for ProGit in your shell.

## Bash

```bash
# Install completion
source ~/ProGit/progit-shell-completions.bash

# Add to ~/.bashrc for persistence
echo "source ~/ProGit/progit-shell-completions.bash" >> ~/.bashrc
```

## Zsh

```bash
# Install completion
source ~/ProGit/progit-shell-completions.zsh

# Add to ~/.zshrc for persistence
echo "source ~/ProGit/progit-shell-completions.zsh" >> ~/.zshrc
```

## Fish

```bash
# Install completion
cp ~/ProGit/progit-shell-completions.fish ~/.config/fish/completions/prog.fish
```

## Usage

After installing, type `prog ` and press Tab:

```bash
prog [Tab]        # Shows: issue mr plugin hook trust sync rebase review tui init
prog plugin [Tab] # Shows: install remove list verify search update submit info
prog plugin install [Tab]  # Shows: syntax-highlight jira-sync gitlab-ci ...
```

## Generating Completions

ProGit can generate completions for your shell:

```bash
prog --completions bash > progit-completions.bash
prog --completions zsh > progit-completions.zsh
prog --completions fish > prog.fish
```
