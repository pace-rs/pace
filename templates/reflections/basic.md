# Activity Reflection

## Time Range

Start: **{{time_range_start}}**

End: **{{time_range_end}}**

## Overview

Total Time Spent: **{{total_time_spent}}**

Total Break Time: **{{total_break_duration}}**

## Summary Groups By Category

{% for key, value in summary_groups_by_category %}

### {{key}}

Duration: **{{value.total_duration}}s**

Break Duration (Count): {{value.total_break_duration}}s ({{value.total_break_count}})

{% for key2, value2 in value.activity_groups_by_description %}

#### {{key2}}

{{value2.description}}

{% endfor %}
{% endfor %}
