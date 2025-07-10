export class InputValidator {
    static readonly MAX_FORM_SIZE = 1024; // 1KB max
    static readonly MAX_FIELD_LENGTH = 512; // 512 chars max per field

    static async validateFormData(
        request: Request
    ): Promise<{valid: boolean; error?: string}> {
        try {
            // Check content length
            const contentLength = request.headers.get("content-length");
            if (contentLength && parseInt(contentLength) > this.MAX_FORM_SIZE) {
                return {valid: false, error: "Request too large"};
            }

            // Check content type
            const contentType = request.headers.get("content-type");
            if (!contentType?.includes("application/x-www-form-urlencoded")) {
                return {valid: false, error: "Invalid content type"};
            }

            return {valid: true};
        } catch (error) {
            return {valid: false, error: "Invalid request format"};
        }
    }

    static validateFieldLength(
        value: string | null | undefined,
        fieldName: string
    ): {valid: boolean; error?: string} {
        if (!value) return {valid: true};

        if (value.length > this.MAX_FIELD_LENGTH) {
            return {
                valid: false,
                error: `Field ${fieldName} exceeds maximum length of ${this.MAX_FIELD_LENGTH} characters`,
            };
        }

        return {valid: true};
    }

    static validateRequiredParams(
        params: Record<string, string | null | undefined>
    ): {valid: boolean; error?: string} {
        for (const [key, value] of Object.entries(params)) {
            if (!value) {
                return {
                    valid: false,
                    error: `Missing required parameter: ${key}`,
                };
            }
        }
        return {valid: true};
    }
}
