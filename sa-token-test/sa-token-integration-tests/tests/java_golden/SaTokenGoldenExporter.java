import cn.dev33.satoken.config.SaCookieConfig;
import cn.dev33.satoken.config.SaTokenConfig;
import cn.dev33.satoken.util.SaFoxUtil;

import java.util.UUID;

/**
 * Emits deterministic behavior metadata from the pinned Java Sa-Token checkout.
 *
 * The random values themselves are intentionally not persisted. The exporter
 * records their observable contracts (length, separators and alphabet).
 */
public final class SaTokenGoldenExporter {

    private SaTokenGoldenExporter() {
    }

    private static boolean isBase62(String value) {
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (!Character.isLetterOrDigit(ch)) {
                return false;
            }
        }
        return true;
    }

    public static void main(String[] args) {
        if (args.length != 1) {
            throw new IllegalArgumentException("expected Java source commit");
        }

        SaTokenConfig config = new SaTokenConfig();
        SaCookieConfig cookie = config.getCookie();
        String uuid = UUID.randomUUID().toString();
        String simpleUuid = UUID.randomUUID().toString().replace("-", "");
        String random32 = SaFoxUtil.getRandomString(32);
        String random64 = SaFoxUtil.getRandomString(64);
        String random128 = SaFoxUtil.getRandomString(128);
        String tik = SaFoxUtil.getRandomString(2)
                + "_" + SaFoxUtil.getRandomString(14)
                + "_" + SaFoxUtil.getRandomString(16) + "__";

        String json = "{\n"
                + "  \"source\": {\"repository\": \"Sa-Token\", \"commit\": \"" + args[0]
                + "\", \"version\": \"1.45.0\"},\n"
                + "  \"config_defaults\": {\n"
                + "    \"token_name\": \"" + config.getTokenName() + "\",\n"
                + "    \"timeout\": " + config.getTimeout() + ",\n"
                + "    \"active_timeout\": " + config.getActiveTimeout() + ",\n"
                + "    \"dynamic_active_timeout\": " + config.getDynamicActiveTimeout() + ",\n"
                + "    \"is_concurrent\": " + config.getIsConcurrent() + ",\n"
                + "    \"is_share\": " + config.getIsShare() + ",\n"
                + "    \"max_login_count\": " + config.getMaxLoginCount() + ",\n"
                + "    \"max_try_times\": " + config.getMaxTryTimes() + ",\n"
                + "    \"is_read_body\": " + config.getIsReadBody() + ",\n"
                + "    \"is_read_header\": " + config.getIsReadHeader() + ",\n"
                + "    \"is_read_cookie\": " + config.getIsReadCookie() + ",\n"
                + "    \"is_lasting_cookie\": " + config.getIsLastingCookie() + ",\n"
                + "    \"is_write_header\": " + config.getIsWriteHeader() + ",\n"
                + "    \"token_style\": \"" + config.getTokenStyle() + "\",\n"
                + "    \"data_refresh_period\": " + config.getDataRefreshPeriod() + ",\n"
                + "    \"token_session_check_login\": " + config.getTokenSessionCheckLogin() + ",\n"
                + "    \"auto_renew\": " + config.getAutoRenew() + ",\n"
                + "    \"cookie_auto_fill_prefix\": " + config.getCookieAutoFillPrefix() + ",\n"
                + "    \"is_print\": " + config.getIsPrint() + ",\n"
                + "    \"is_log\": " + config.getIsLog() + "\n"
                + "  },\n"
                + "  \"java_cookie_defaults\": {\n"
                + "    \"domain_is_null\": " + (cookie.getDomain() == null) + ",\n"
                + "    \"path_is_null\": " + (cookie.getPath() == null) + ",\n"
                + "    \"secure\": " + cookie.getSecure() + ",\n"
                + "    \"http_only\": " + cookie.getHttpOnly() + ",\n"
                + "    \"same_site_is_null\": " + (cookie.getSameSite() == null) + "\n"
                + "  },\n"
                + "  \"token_styles\": {\n"
                + "    \"uuid\": {\"length\": " + uuid.length()
                + ", \"hyphens\": 4},\n"
                + "    \"simple_uuid\": {\"length\": " + simpleUuid.length()
                + ", \"base62\": " + isBase62(simpleUuid) + "},\n"
                + "    \"random_32\": {\"length\": " + random32.length()
                + ", \"base62\": " + isBase62(random32) + "},\n"
                + "    \"random_64\": {\"length\": " + random64.length()
                + ", \"base62\": " + isBase62(random64) + "},\n"
                + "    \"random_128\": {\"length\": " + random128.length()
                + ", \"base62\": " + isBase62(random128) + "},\n"
                + "    \"tik\": {\"length\": " + tik.length()
                + ", \"separator_indexes\": [2, 17], \"suffix\": \"__\", \"segments\": [2, 14, 16]}\n"
                + "  },\n"
                + "  \"apikey_defaults\": {\"prefix\": \"AK-\", \"random_length\": 36,"
                + " \"timeout\": 2592000, \"record_index\": true}\n"
                + "}\n";
        System.out.print(json);
    }
}
