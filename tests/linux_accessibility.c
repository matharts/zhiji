/* 在隔离的 X11/D-Bus 会话中执行原生 AT-SPI 冒烟检查。 */
#include <atspi/atspi.h>
#include <glib.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static pid_t child = -1;

/* 限制递归深度；返回的匹配节点持有独立引用，由调用方释放。 */
static AtspiAccessible *find_name(AtspiAccessible *node, const char *name, int depth) {
    if (!node || depth > 25) return NULL;
    GError *error = NULL;
    char *actual = atspi_accessible_get_name(node, &error);
    if (error) { g_clear_error(&error); g_free(actual); return NULL; }
    gboolean match = actual && strcmp(actual, name) == 0;
    g_free(actual);
    if (match) return g_object_ref(node);
    int count = atspi_accessible_get_child_count(node, &error);
    if (error) { g_clear_error(&error); return NULL; }
    for (int i = 0; i < count; ++i) {
        AtspiAccessible *next = atspi_accessible_get_child_at_index(node, i, &error);
        if (error) { g_clear_error(&error); continue; }
        AtspiAccessible *result = find_name(next, name, depth + 1);
        if (next) g_object_unref(next);
        if (result) return result;
    }
    return NULL;
}

/* 刷新桌面缓存并仅匹配本测试子进程，返回的应用引用由调用方释放。 */
static AtspiAccessible *application(void) {
    AtspiAccessible *desktop = atspi_get_desktop(0);
    if (!desktop) return NULL;
    atspi_accessible_clear_cache(desktop);
    GError *error = NULL;
    int count = atspi_accessible_get_child_count(desktop, &error);
    if (error) { g_clear_error(&error); g_object_unref(desktop); return NULL; }
    AtspiAccessible *result = NULL;
    for (int i = 0; i < count; ++i) {
        AtspiAccessible *app = atspi_accessible_get_child_at_index(desktop, i, &error);
        if (error) { g_clear_error(&error); continue; }
        if (app && atspi_accessible_get_process_id(app, &error) == (guint)child && !error) { result = app; break; }
        g_clear_error(&error);
        if (app) g_object_unref(app);
    }
    g_object_unref(desktop);
    return result;
}

/* 有界等待控件出现或消失；消失时返回应用引用，进程退出及超时均失败。 */
static AtspiAccessible *wait_for(const char *name, gboolean absent) {
    gint64 deadline = g_get_monotonic_time() + 15 * G_TIME_SPAN_SECOND;
    do {
        int status;
        if (waitpid(child, &status, WNOHANG) == child) { child = -1; fprintf(stderr, "desktop process exited: %d\n", status); return NULL; }
        AtspiAccessible *app = application();
        if (app) {
            atspi_accessible_clear_cache(app);
            AtspiAccessible *item = find_name(app, name, 0);
            if (absent && !item) return app;
            g_object_unref(app);
            if (!absent && item) return item;
            if (item) g_object_unref(item);
        }
        g_usleep(200000);
    } while (g_get_monotonic_time() < deadline);
    fprintf(stderr, "accessibility element %s: %s\n", absent ? "still present or app unavailable" : "missing", name);
    return NULL;
}

/* 等待控件后执行首个可访问动作，释放接口与节点引用并报告底层错误。 */
static gboolean press(const char *name) {
    AtspiAccessible *node = wait_for(name, FALSE);
    if (!node) return FALSE;
    AtspiAction *action = atspi_accessible_get_action_iface(node);
    GError *error = NULL;
    gboolean ok = action && atspi_action_get_n_actions(action, &error) > 0 && !error && atspi_action_do_action(action, 0, &error) && !error;
    if (!ok) fprintf(stderr, "cannot activate %s: %s\n", name, error ? error->message : "no action");
    g_clear_error(&error);
    if (action) g_object_unref(action);
    g_object_unref(node);
    return ok;
}

/* 核验可访问节点的存在状态，并按需检查滑块角色。 */
static gboolean check(const char *name, gboolean absent, gboolean slider) {
    AtspiAccessible *node = wait_for(name, absent);
    if (!node) return FALSE;
    GError *error = NULL;
    gboolean ok = !slider || atspi_accessible_get_role(node, &error) == ATSPI_ROLE_SLIDER;
    if (error || !ok) fprintf(stderr, "%s is not an accessible slider\n", name);
    g_clear_error(&error);
    g_object_unref(node);
    return ok;
}

/* 在隔离总线启用辅助功能并检查桌面交互，结束时仅回收本测试子进程。 */
int main(int argc, char **argv) {
    if (argc != 2) { fprintf(stderr, "usage: linux_accessibility <desktop-binary>\n"); return 2; }
    const char *properties[] = { "IsEnabled", "ScreenReaderEnabled" };
    for (int i = 0; i < 2; ++i) {
        char *args[] = { "gdbus", "call", "--session", "--dest", "org.a11y.Bus", "--object-path", "/org/a11y/bus", "--method", "org.freedesktop.DBus.Properties.Set", "org.a11y.Status", (char *)properties[i], "<true>", NULL };
        int status = 0; GError *error = NULL;
        if (!g_spawn_sync(NULL, args, NULL, G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL, NULL, NULL, NULL, NULL, &status, &error) || !g_spawn_check_wait_status(status, &error)) {
            fprintf(stderr, "cannot enable accessibility: %s\n", error ? error->message : "unknown error"); g_clear_error(&error); return 1;
        }
    }
    if (atspi_init() != 0) { fprintf(stderr, "cannot initialize AT-SPI\n"); return 1; }
    child = fork();
    if (child == 0) { execl(argv[1], argv[1], (char *)NULL); _exit(127); }
    if (child < 0) { perror("fork"); atspi_exit(); return 1; }
    gboolean ok = press("收起右侧面板") && check("性別 女", TRUE, FALSE) &&
        check("导出图片", FALSE, FALSE) && press("展开右侧面板") &&
        check("性別 女", FALSE, FALSE) && check("导出图片", FALSE, FALSE) &&
        press("设置") && press("外观") && press("祿飞宫颜色") &&
        check("色相", FALSE, TRUE) && check("透明度", FALSE, TRUE) &&
        press("返回排盘信息") && check("导出图片", FALSE, FALSE);
    if (child > 0) {
        kill(child, SIGTERM);
        gint64 deadline = g_get_monotonic_time() + 5 * G_TIME_SPAN_SECOND;
        int status;
        while (waitpid(child, &status, WNOHANG) == 0 && g_get_monotonic_time() < deadline)
            g_usleep(20000);
        if (waitpid(child, &status, WNOHANG) == 0) {
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
        }
    }
    atspi_exit();
    if (!ok) return 1;
    puts("Linux accessibility: sidebar toggle, export control and color sliders available");
    return 0;
}
