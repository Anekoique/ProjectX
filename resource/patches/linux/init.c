// Minimal init for xemu Linux — uses raw syscalls, no libc, no FP.
// Provides a basic shell that can run statically linked binaries.

typedef unsigned long size_t;
typedef long ssize_t;

// Raw syscall wrapper
static long syscall1(long nr, long a0) {
    register long _a0 __asm__("a0") = a0;
    register long _nr __asm__("a7") = nr;
    __asm__ volatile("ecall" : "+r"(_a0) : "r"(_nr) : "memory");
    return _a0;
}
static long syscall2(long nr, long a0, long a1) {
    register long _a0 __asm__("a0") = a0;
    register long _a1 __asm__("a1") = a1;
    register long _nr __asm__("a7") = nr;
    __asm__ volatile("ecall" : "+r"(_a0) : "r"(_a1), "r"(_nr) : "memory");
    return _a0;
}
static long syscall3(long nr, long a0, long a1, long a2) {
    register long _a0 __asm__("a0") = a0;
    register long _a1 __asm__("a1") = a1;
    register long _a2 __asm__("a2") = a2;
    register long _nr __asm__("a7") = nr;
    __asm__ volatile("ecall" : "+r"(_a0) : "r"(_a1), "r"(_a2), "r"(_nr) : "memory");
    return _a0;
}
static long syscall5(long nr, long a0, long a1, long a2, long a3, long a4) {
    register long _a0 __asm__("a0") = a0;
    register long _a1 __asm__("a1") = a1;
    register long _a2 __asm__("a2") = a2;
    register long _a3 __asm__("a3") = a3;
    register long _a4 __asm__("a4") = a4;
    register long _nr __asm__("a7") = nr;
    __asm__ volatile("ecall" : "+r"(_a0) : "r"(_a1), "r"(_a2), "r"(_a3), "r"(_a4), "r"(_nr) : "memory");
    return _a0;
}

#define __NR_write    64
#define __NR_read     63
#define __NR_exit     93
#define __NR_mkdirat  34
#define __NR_mount    40
#define __NR_getdents64 61
#define __NR_chdir    49
#define __NR_getcwd   17
#define __NR_openat   56
#define __NR_close    57
#define __NR_uname    160
#define AT_FDCWD      (-100)

static ssize_t write(int fd, const void *buf, size_t n) {
    return syscall3(__NR_write, fd, (long)buf, n);
}
static ssize_t read(int fd, void *buf, size_t n) {
    return syscall3(__NR_read, fd, (long)buf, n);
}
static void puts(const char *s) {
    size_t n = 0;
    while (s[n]) n++;
    write(1, s, n);
}

static int streq(const char *a, const char *b) {
    while (*a && *b && *a == *b) { a++; b++; }
    return *a == *b;
}

struct linux_dirent64 {
    unsigned long long d_ino;
    long long d_off;
    unsigned short d_reclen;
    unsigned char d_type;
    char d_name[];
};

struct utsname {
    char sysname[65];
    char nodename[65];
    char release[65];
    char version[65];
    char machine[65];
};

static void cmd_uname(void) {
    struct utsname u;
    if (syscall1(__NR_uname, (long)&u) == 0) {
        puts(u.sysname); puts(" ");
        puts(u.nodename); puts(" ");
        puts(u.release); puts(" ");
        puts(u.machine); puts("\n");
    }
}

static void cmd_ls(void) {
    char dirbuf[1024];
    int fd = syscall3(__NR_openat, AT_FDCWD, (long)".", 0x10000 /* O_DIRECTORY */);
    if (fd < 0) { puts("ls: cannot open dir\n"); return; }
    long n;
    while ((n = syscall3(__NR_getdents64, fd, (long)dirbuf, sizeof(dirbuf))) > 0) {
        long off = 0;
        while (off < n) {
            struct linux_dirent64 *d = (void *)(dirbuf + off);
            puts(d->d_name); puts("  ");
            off += d->d_reclen;
        }
    }
    puts("\n");
    syscall1(__NR_close, fd);
}

static void cmd_pwd(void) {
    char buf[256];
    if (syscall2(__NR_getcwd, (long)buf, sizeof(buf)) > 0) {
        puts(buf); puts("\n");
    }
}

static void cmd_cd(const char *path) {
    if (syscall1(__NR_chdir, (long)path) < 0)
        puts("cd: no such directory\n");
}

static void cmd_cat(const char *path) {
    int fd = syscall3(__NR_openat, AT_FDCWD, (long)path, 0);
    if (fd < 0) { puts("cat: cannot open file\n"); return; }
    char buf[512];
    long n;
    while ((n = syscall3(__NR_read, fd, (long)buf, sizeof(buf))) > 0)
        write(1, buf, n);
    syscall1(__NR_close, fd);
}

static void cmd_echo(const char *arg) {
    if (arg) puts(arg);
    puts("\n");
}

static void cmd_help(void) {
    puts("Built-in commands: ls pwd cd cat echo uname help poweroff\n");
}

// Trim trailing newline
static void chomp(char *s) {
    size_t n = 0;
    while (s[n]) n++;
    if (n > 0 && s[n-1] == '\n') s[n-1] = 0;
}

// Find first space, split into cmd and arg
static char *split_arg(char *s) {
    while (*s && *s != ' ') s++;
    if (*s == ' ') { *s = 0; return s + 1; }
    return 0;
}

void _start(void) {
    // Mount essential filesystems
    syscall3(__NR_mkdirat, AT_FDCWD, (long)"/proc", 0755);
    syscall3(__NR_mkdirat, AT_FDCWD, (long)"/sys", 0755);
    syscall3(__NR_mkdirat, AT_FDCWD, (long)"/dev", 0755);
    syscall5(__NR_mount, (long)"proc", (long)"/proc", (long)"proc", 0, 0);
    syscall5(__NR_mount, (long)"sysfs", (long)"/sys", (long)"sysfs", 0, 0);
    syscall5(__NR_mount, (long)"devtmpfs", (long)"/dev", (long)"devtmpfs", 0, 0);

    puts("\nWelcome to xemu Linux!\n\n");

    char buf[256];
    for (;;) {
        puts("# ");
        ssize_t n = read(0, buf, sizeof(buf) - 1);
        if (n <= 0) continue;
        buf[n] = 0;
        chomp(buf);
        if (buf[0] == 0) continue;

        char *arg = split_arg(buf);

        if (streq(buf, "ls"))           cmd_ls();
        else if (streq(buf, "pwd"))   cmd_pwd();
        else if (streq(buf, "cd"))    cmd_cd(arg ? arg : "/");
        else if (streq(buf, "echo"))  cmd_echo(arg);
        else if (streq(buf, "cat"))   { if (arg) cmd_cat(arg); else puts("usage: cat <file>\n"); }
        else if (streq(buf, "uname")) cmd_uname();
        else if (streq(buf, "help"))  cmd_help();
        else if (streq(buf, "poweroff") || streq(buf, "halt") || streq(buf, "exit")) {
            puts("System halting.\n");
            syscall3(142 /* __NR_reboot */, 0xfee1dead, 0x28121969, 0x4321fedc /* POWER_OFF */);
        }
        else { puts(buf); puts(": command not found\n"); }
    }
}
