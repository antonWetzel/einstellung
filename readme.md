# Einstellung

- A simple tool to synchronize your local configuration files
- `einstellung read` to read changes to configuration files 
- `einstellung write` to write changes to configuration files 

## Setup 

1. Install: Setup Rust and run `cargo install --git=https://github.com/antonWetzel/einstellung`
2. Configuration: Create the file `.einstellung` in the directory you want to synchronize
3. Usage: Run `einstellung read` and `einstellung write`

## Configuration

Every line in `.einstellung` contains the file you want to synchronize, followed by a list of search locations  

### Configuration Example

```text
# Empty lines or starting with '#' are comments
# sync-file search-location-1 search-location-2 ...
.zshrc ~/.zshrc
zed.json ~/.config/zed/settings.json
starship.toml ~/.config/starship.toml
```

## Usage

### Usage for Read

1. The synchronized file is compared line by line to every file found at the search locations
2. Choose to include added or removed lines
3. Choose to save the new synchronized file
4. Use your preferred version control system to save the changes

### Usage for Write

1. The synchronized file is compared to every file found at the search locations
2. Choose to update the files at the search locations with the content of the synchronized file
