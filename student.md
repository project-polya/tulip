# Student Configuration

Student should be a `init.yml` in their project directory. It will be scanned by the server:

```yaml
build_script: build.sh
run_script: run.sh
public_key: key.pub // optional
report: report.pdf // optional	
notification: "my project may use x server"
```

If a public key is provided, the student report will be encrypted by the server.

