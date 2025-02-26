//! This is a library of functions that can be used to plot data on charts.
//! Data from the platform can come in many different formats each of which
//! is represented differently on charts.  These functions take input data
//! from serialization/aggregation functions and plot it on HighCharts charts.

"use strict";
/*jslint browser: true*/ /*global Highcharts, $*/

/// Takes the name of the indicator as an argument and plots the corresponding
/// chart.
function matchPlot(indicator, chart, data) {
  switch(indicator) {
    case "ticks":
      simpleLinearPlot(chart, data);
      break;
    case "bidask":
      bollingerStylePlot(chart, data);
  }
}

/// Plots a simple linear set of data consisting of a single array of points
/// with increasing timestamps.
function simpleLinearPlot(chart, data) {
  var series = {data: data.data, name: `${data.name}`};
  chart.addSeries(series);
}

/// Plots a set of data representing deviation from zero with the area under
/// the curve being shaded.
function areafillLinearPlot(chart, data) {

}

/// Plots a Bollinger Bands style of data consisting of two arrays of data
/// representing the upper and lower bounds, shadingthe area in the middle.
function bollingerStylePlot(chart, data) {
  // var new_options = chart.options;
  // new_options.chart.type = "arearange";
  // chart = new Highcharts.Chart(new_options);

  var series = {data: data.data, name: data.name, type: "arearange"};
  chart.addSeries(series);
}

/// Plots opened trades and currently open positions on a chart.  These are
/// represented by lines from the price the trade was opened at to where the
/// trade was closed at colored to represent the success of the trade.
function tradesPlot(chart, data) {

}
