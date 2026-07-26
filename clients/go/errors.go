package crawlkit

import "fmt"

type APIError struct {
	StatusCode int
	Message    string
}

func (e *APIError) Error() string {
	return fmt.Sprintf("HTTP %d: %s", e.StatusCode, e.Message)
}

func (e *APIError) Is(target error) bool {
	t, ok := target.(*APIError)
	if !ok {
		return false
	}
	return e.StatusCode == t.StatusCode
}

func IsAuthError(err error) bool {
	apiErr, ok := err.(*APIError)
	return ok && apiErr.StatusCode == 401
}

func IsNotFoundError(err error) bool {
	apiErr, ok := err.(*APIError)
	return ok && apiErr.StatusCode == 404
}

func IsRateLimitError(err error) bool {
	apiErr, ok := err.(*APIError)
	return ok && apiErr.StatusCode == 429
}

func IsValidationError(err error) bool {
	apiErr, ok := err.(*APIError)
	return ok && apiErr.StatusCode == 400
}
