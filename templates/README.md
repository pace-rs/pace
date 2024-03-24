# Templating

These are the templates for the project. They use the Tera templating engine. If
you want to learn more about Tera, you can check out the
[Tera documentation](https://keats.github.io/tera/docs/#introduction).

## Template files

You can find the following templates in this directory:

- `reflections/basic.html`: A basic template for the reflections.
- `reflections/basic.md`: A basic template for the reflections in markdown.
- `reflections/pace_report_json.html`: A template for using the exported JSON
  data.

## Generating the reflections

To generate the reflections, you can call the following command:

```console
pace reflection -o template -t templates/reflections/basic.md -e test.md today
```
