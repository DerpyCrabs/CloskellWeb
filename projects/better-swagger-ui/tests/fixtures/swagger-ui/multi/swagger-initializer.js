window.onload = function () {
  window.ui = SwaggerUIBundle({
    urls: [
      { url: "/openapi/multi-definition-a.json", name: "API A" },
      { url: "/openapi/multi-definition-b.json", name: "API B" },
    ],
    dom_id: "#swagger-ui",
  })
}
