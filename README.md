# Kairos

Kairos is a link monitor that logs, and alerts you to, new links on websites that you specify in its configuration.
Its motivating use case is to monitor job postings.

(And to give me an opportunity to practice async Rust.)

## Features

+ [X] Configure in plain text via a single TOML file.
+ [X] Store observed links in a SQLite database.
+ [X] Describe monitored links with
  - [X] CSS selectors or
  - [X] XPath expressions.
+ [X] Send push notifications via Pushover.
+ [ ] Serve a web UI to view observed links.

## Installation

If you have Cargo, then run:

```sh
cargo install --locked --git https://github.com/gn0/kairos.git
```

If `$HOME/.cargo/bin` is not in your `PATH` environment variable, then you also need to run:

```sh
export PATH=$HOME/.cargo/bin:$PATH
```

To make this setting permanent:

```sh
echo 'export PATH=$HOME/.cargo/bin:$PATH' >> $HOME/.bashrc  # If using bash.
echo 'export PATH=$HOME/.cargo/bin:$PATH' >> $HOME/.zshrc   # If using zsh.
```

## Usage

### Launch Kairos

When you launch Kairos, you need to specify the path to the configuration file:

```sh
kairos --verbose --config path/to/config.toml
```

An example configuration is shown in [`example/config.toml`](./example/config.toml).

It is convenient to automatically launch Kairos as a user service with systemd.

1. Run `mkdir -p ~/.config/systemd/user`.
2. Copy `systemd/kairos.service` from this repository to `~/.config/systemd/user/`.
   - If you did not install Kairos with Cargo, then edit `kairos.service` to use the correct path to the executable.
   - The service will expect your configuration file to be at `~/.config/kairos/config.toml`.
3. Run `loginctl enable-linger $USER`.
   - This tells systemd to start your services, including Kairos, on boot rather than on login.
4. Run `systemctl --user enable kairos`.

Now Kairos should be operational.
The status of the service can be checked with `systemctl --user status kairos`.
Logs can be inspected with `journalctl --user -u kairos`.

### Sandboxing

XPath expressions are supported via libxml2 which is a library written in C, maintained by a volunteer.
Although the library is widely used (`apt-cache rdepends libxml2 | grep '^  ' | wc -l` indicates 787 packages on Ubuntu 24.04), the maintainer himself states that it is badly tested and can be expected to be full of security bugs.
He writes that [it is foolish to use it with untrusted inputs](https://gitlab.gnome.org/GNOME/libxml2/-/issues/913#note_2439345), which means all of the internet.

Kairos does not invoke libxml2 if the configuration uses no XPath expressions, only CSS selectors.

If you do need XPath expressions, you can prevent unpleasant surprises by sandboxing Kairos with [Bubblewrap](https://wiki.archlinux.org/title/Bubblewrap).
The systemd service file in this repository, [`systemd/kairos.service`](./systemd/kairos.service), shows an example of how to do this.

### Reload the configuration

Kairos reloads the configuration if it receives a hangup signal.
You can send a hangup signal by running

```sh
# Replace `PID` with the PID of Kairos.
kill -HUP PID
```

If you set Kairos up with systemd, then you don't need to find the PID because `systemctl` can send a hangup signal for you:

```sh
systemctl --user reload kairos
```

**Note:** Reloading via `systemctl` won't work if Kairos is sandboxed with Bubblewrap.
In this case, systemd will send the hangup signal to Bubblewrap, not to Kairos.
Instead of reloading via `systemctl`, find the PID of Kairos by running `systemctl --user status kairos`, and send the signal to the process manually with `kill -HUP PID`.

### Cancel currently running collection

Kairos cancels the currently running collection (if any) if it receives a USR1 signal.
You can send this signal by running

```sh
# Replace `PID` with the PID of Kairos.
kill -USR1 PID
```

### Testing CSS selectors

(These instructions assume that you're using [Firefox](https://www.firefox.com/).)

One of the two ways in which Kairos finds the elements to monitor is using [CSS selectors](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_selectors).
CSS selectors describe elements by their attributes, child elements, and parent elements.
To figure out what selectors will do the job for you, you need to inspect the structure of the page.

After right-clicking on an element that you'd like to monitor, click on "Inspect."
This will pull up the "Developer Tools."
Look at
+ whether it has characteristic attributes that you could filter for, or
+ whether its parent elements have anything that you could filter for.

Suppose that you'd like to try the CSS selector `a[href^='/careers']:not([href='/careers/feed'])` to filter for the tags you're interested in.

1. If the "Developer Tools" are not already open, then press `F12` to pull them up.
2. Select the "Console" tab.
3. Enter the following in the console and confirm that the output is the number of elements that you're interested in on the page:
   ```javascript
   $$("a[href^='/careers']:not([href='/careers/feed'])").length
   ```

## License

Kairos is distributed under the GNU Affero General Public License (AGPL), version 3.
See the file [LICENSE](./LICENSE) for more information.

