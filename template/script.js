const data = JSON.parse(document.getElementById("data").textContent);

const commits = data.commits;
const benches = data.benches;
const labels = commits.map((c) => c.sha);

const datasets = [];
const colorPalette = [
  "#3366cc",
  "#dc3912",
  "#ff9900",
  "#109618",
  "#990099",
  "#0099c6",
  "#dd4477",
  "#66aa00",
  "#b82e2e",
  "#316395",
];

let colorIndex = 0;
for (const [benchName, scenarios] of Object.entries(benches)) {
  const baseColor = colorPalette[colorIndex % colorPalette.length];
  colorIndex++;

  for (const [scenario, line] of Object.entries(scenarios)) {
    let borderDash;
    switch (scenario) {
      case "cold":
        break;
      case "memo":
        borderDash = [15, 15];
        break;
      case "update":
        borderDash = [15, 5, 5, 5];
        break;
      default:
        borderDash = [3, 3];
    }
    const ds = {
      label: `${benchName}/${scenario}`,
      data: line,
      borderColor: baseColor,
      borderDash: borderDash,
      spanGaps: false,
    };
    datasets.push(ds);
  }
}

const ctx = document.getElementById("chart").getContext("2d");
new Chart(ctx, {
  type: "line",
  data: { labels: labels, datasets: datasets },
  options: {
    responsive: true,
    interaction: { mode: "index", intersect: false },
    plugins: {
      tooltip: {
        callbacks: {
          title(items) {
            return commits[items[0].dataIndex].title;
          },
        },
      },
      legend: {
        onClick(e, legendItem, legend) {
          const clickedLabel =
            legend.chart.data.datasets[legendItem.datasetIndex].label || "";
          const bench = clickedLabel.split("/")[0];
          if (e.ctrlKey || e.metaKey) {
            const vis = legend.chart.isDatasetVisible(legendItem.datasetIndex);
            legend.chart.setDatasetVisibility(legendItem.datasetIndex, !vis);
            legend.chart.update();
            return;
          }
          const indices = legend.chart.data.datasets
            .map((d, idx) => ({ idx, label: d.label || "" }))
            .filter((d) => d.label.split("/")[0] === bench)
            .map((d) => d.idx);
          if (indices.length === 0) return;
          const anyVisible = indices.some((i) =>
            legend.chart.isDatasetVisible(i)
          );
          for (const i of indices) {
            legend.chart.setDatasetVisibility(i, !anyVisible);
          }
          legend.chart.update();
        },
      },
    },
    scales: {
      x: { title: { display: true, text: "Commit" } },
      y: { title: { display: true, text: "Time (us)" } },
    },
  },
});
