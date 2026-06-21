package identitysdk

type Client struct {
	BaseURL string
}

func NewClient(baseURL string) Client {
	return Client{BaseURL: baseURL}
}

func (client Client) HealthPath() string {
	return client.BaseURL + "/health"
}
