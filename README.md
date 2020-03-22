# Tulip

![logo](logo.png)

Tulip is a part of [Project Polya](https://github.com/project-polya).

It is the endpoint program, which is responsible for creating the judge environment and running the student project.

## Requirements for Use

- `Linux` is required.

- [`nutshell`](https://crates.io/crates/nutshell) is installed and put in the `PATH`.
- One of `aria2c` and `wget` is ready.
- `openssl` is usable for `https` .
- `sudo` is within the `PATH` and you have the right to become the root.
- `systemd-nspawn`  and `firejail` is required for sandbox.

## Requirements for Build

- `Linux` is required.
- Make sure your rust tool-chain is functioning.
- `cmake` is installed and `c/c++` development environment is ready to be invoked by `cargo`.

## Environment Variables

- The following variables are optional and they can also be passed as command-line arguments:

  - `TULIP_LOG_LEVEL` The log level of `tulip`, set to `info` by default. It is recommended to set a level finer than `warn`. Possible values:
    - off
  
    - error
  
    - warn
  
    - info
    
    - debug
    
    - trace
  - `TULIP_REPORT_READER` The application to open student report, set to `xdg-open` by default.
  - `NUTSHELL_BIN` Path to the `nutshell` executable, set to `nutshell` by default.
  - `TULIP_DIR` The work directory of `tulip`, set `.tulip` by default
  - `TULIP_MOUNT_DIR` The mount directory of the temporary `overlayfs`. **Attention: This is set to `\mnt` be default**
- The following system wise variables are used:
  
  - `EDITOR` will be used when editing configurations if it is set
- The following variables must be provided or passed as command-line arguments:
  - `TULIP_SERVER` will be used when registering if it is set
  - `TULIP_TOKEN` will be used when registering if it is set



## About the Status of Grading

- Once a project is fetched it is locked by the fetcher.

- The project can be unlocked by `commit/skip/revoke`, where the last operation is included in `clean-all`.

- `pull`  is always allowed. However, `pull` will not lock the project, you will always need to use `--override` flag to update the remote status if the current project is pulled rather than fetched.

- `commit`  can be used to commit the current project. However, if a previous submission exists, `--override` flag must be set.

- `fetch` will automatically get the next untouched project.

- `skip` can ignore the current project and unlock it. However, once a project is `skipped`, it can only be pulled and will not be put into the fetch-able list.



## About the Procedure of Grading

- You need to register a UUID at the very beginning.
- You should pull an image before grading. The default `pull-image` will download the image and the global configurations.  However, both of them can be set locally. 

  - `image` can be set by putting the image  to  `<workdir>/image/image.sfs` and invoking the `pull-image` subcommand with `--local-set` 

  - `global-config` can be set by `tulip status edit-global`
- Now, you can either pull or fetch a project. When pulling, you can use `tulip status remote [--detail]` or `tulip status remote-id --id <student id>` . If you experience a download error when fetching the project file or you just want to update the current project info, you can use `tulip fetch --download-only`
- How, you can initialize the overlay. The student files are `rsynced` into the `/data` directory in the chroot environment.
- You can run `build` subcommand to build the project.
- You can run `run` subcommand to run the project.
- You can use `comment` subcommand to leave a comment.
- You can use `grade -s <score>` subcommand to grade the project.
- You can use `mark [-r]` subcommand to mark/unmark the project.
- You can use `commit/skip` subcommand to submit/skip the project.
- You can use `report` subcommand to read the report of the student.
- During the whole procedure, you can use `enter-sandbox` to enter the sandbox, both`firejail` and `systemd-nspawn` .
- After a local project is set, building-running-report process can be invoked in a whole by the `auto-current` subcommand.

## Notice

There are a lot of more details: for example, you can force to rebuild, force to re-grade, directly edit the status, etc. All the features are described in detail in the CLI. Feel free to invoke the CLI with `--help` whenever you feel confused.

## Configuration

- [global configuration](global.md)
- [student configuration](student.md)



