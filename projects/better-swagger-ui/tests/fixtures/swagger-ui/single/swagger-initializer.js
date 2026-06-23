window.onload = function () {
  window.ui = SwaggerUIBundle({
    url: "/openapi/minimal.json",
    dom_id: "#swagger-ui",
  })
}
