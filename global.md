# Global Configuration

This configuration should be set in the server of Project Polya.

```jsonc
{
  "systemd_nspawn": {
    "pid2": true, // create a virtual init in the sandox as pid1  
    "env": [ 
    	{"name": "example", "value": "example"}
    ], 
    "work_path": null, // change dir after enter the sandbox, set a string here if you need this
    "syscall": [
    	{"name": "mmap", "permit": true},
    	{"name": "clone", "permit": false},
    ],
    "capacity": [], // allowed linux capacities, use string format 
    "capacity_drop": [], // dropped linux capacities, use string format 
    "no_new_privileges": false, // set no_new_privileges environment variables
    "no_network": false, // disable network
    "limit": null, // can be null, see below
    "shell": null // default shell, set a string here if you want.
  },
  "firejail": {
    "timeout": null, // can be null, see below
    "syscall": [],
    "shell": null,
    "nice": null, // set nice value
    "function": { // disable funtions
      "nou2f": false,
      "novideo": false,
      "no3d": false,
      "noautopulse": false,
      "nogroups": false,
      "nonewprivs": false,
      "nodvd": false,
      "nodbus": false,
      "nonet": false
    },
    "mac": null, // mac address
    "dns": null, // dns server 
    "nodefault": false, // cancel default profile
    "allow_debuggers": false, // enable denuggers
    "limit": null, // see below
    "capacity": [],
    "capacity_drop": [],
    "with_profile": null, // set a profile, relative to chroot
    "has_x": false, // adjust xhost before running
    "env": [],
    "env_remove": [],
    "whilelist": [] // path to allow edit, relative to the chroot
  },
  "notification": "", // global notification, will be showed to teacher
  "max_grade": 0, // grading range
  "stdin": null // set a stdin file relative to chroot
}
```



The limit part can be null or it can be set as the following (each field is also nullable):

```jsonc
}
    "mem_limit": 536870912, // memory size in byte
    "nofile_limit": 2, // number of file that can be created 
    "filesize_limit": 1024, // file size limit in byte
    "process_limit": 5, // new process limit
    "sigpending_limit": 10, // signal pending limit
    "cpu_nums": 2, // num of cpu cores, this limit is achieved by cpu affinity
}
```

The timeout take the form of

```jsonc
{
	"hour": 0, "minute": 1, "second": 30
}
```

**Attention: The path mention above must be relative to the mount point. For example, if you want to add /var in the chroot whilelist, the you should add `var`, not `/var`**.

