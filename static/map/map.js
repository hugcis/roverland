var getUrl = window.location;
var baseUrl = getUrl.protocol + "//" + getUrl.host;
var map = L.map('map').setView([39.74739, -105], 13);

var tiles = L.tileLayer('https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token=pk.eyJ1IjoibWFwYm94IiwiYSI6ImNpejY4NXVycTA2emYycXBndHRqcmZ3N3gifQ.rJcFIG214AriISLbB6B5aw', {
    maxZoom: 18,
    attribution: 'Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, ' +
        'Imagery © <a href="https://www.mapbox.com/">Mapbox</a>',
    id: 'mapbox/dark-v9',
    tileSize: 512,
    zoomOffset: -1
}).addTo(map);
let dateStart = moment().startOf('day');
let dateEnd = dateStart.clone().add(1, "days");
let overlayLayer = null;

const ctx = document.getElementById('chart').getContext('2d');
const myChart = new Chart(ctx, {
    type: 'scatter',
    data: {
        labels: [],
        datasets: [{
                xAxisID: "xAxes",
                yAxisID: "y",
                label: 'Battery level',
                data: [],
                borderColor: 'rgb(243, 102, 102)',
                fill: false,
                tension: 1.,
                borderWidth: 2.,
                pointRadius: 0.,
                showLine: true,
            },
            // {
            //     type: "scatter",
            //     label: "State",
            //     xAxisID: "xAxes",
            //     data: [],
            //     barPercentage: 10.,
            // },
        ]
    },
    options: {
        interaction: {
            intersect: false,
            mode: 'point',
        },
        scales: {
            y: {
                beginAtZero: true
            },
            xAxes: {
                ticks: {
                    callback: function(label, index, ticks) {
                        return moment(label).format("HH:mm");
                    }
                }
            },
        },
        plugins: {
            tooltip: {
                callbacks: {
                    label: function(context) {
                        var label = moment(parseInt(context.label.replaceAll(",", ""))).format("HH:mm") + " " + context.formattedValue;
                        return label; + ': (' + tooltipItem.xLabel + ', ' + tooltipItem.yLabel + ')';
                    }
                }
            }
        }
    }
});


L.Control.Command = L.Control.extend({
    options: {
        position: 'topleft',
    },
    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', PreviousDate);

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Previous";
        return controlDiv;
    }
});
L.control.command = function(options) {
    return new L.Control.Command(options);
};
L.control.command({}).addTo(map);

L.Control.CommandNext = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', NextDate);

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Next";
        return controlDiv;
    }
});
L.control.commandNext = function(options) {
    return new L.Control.CommandNext(options);
};
L.control.commandNext({}).addTo(map);

L.Control.DatePicker = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)

        controlDiv.innerHTML = `<div id="date-range-picker" style="background-color: white;">
            <h4>Date range</h4>
            <center>
              <input type="text" name="daterange-picker" class="form-control">
            </center>
          </div>`;
        return controlDiv;
    }
});
L.control.datePicker = function(options) {
    return new L.Control.DatePicker(options);
};
L.control.datePicker({}).addTo(map);

$(function() {
    $('input[name="daterange-picker"]').daterangepicker({
        ranges: {
            'Today': [moment(), moment().add(1, "days")],
            'Yesterday': [moment().subtract(1, 'days'), moment()],
            'Last 7 Days': [moment().subtract(6, 'days'), moment()],
            // 'Last 30 Days': [moment().subtract(29, 'days'), moment()],
            // 'This Month': [moment().startOf('month'), moment().endOf('month')],
            // 'Last Month': [moment().subtract(1, 'month').startOf('month'), moment().subtract(1, 'month').endOf('month')]
        },
        "startDate": moment(),
        "endDate": moment(),
        "opens": "left"
    }, function(start, end, label) {
        dateStart = start;
        dateEnd = end;
        console.log('New date range selected: ' + dateStart.format('YYYY-MM-DD') + ' to ' + dateEnd.format('YYYY-MM-DD') + ' (predefined range: ' + label + ')');
        updateData(dateStart, dateEnd);
    })
});


function NextDate() {
    dateStart.add(1, 'days');
    dateEnd.add(1, 'days');
    updateData(dateStart, dateEnd);
}

function PreviousDate() {
    dateStart.add(-1, 'days');
    dateEnd.add(-1, 'days');
    updateData(dateStart, dateEnd);
}


function onEachFeature(feature, layer) {
    var popupContent = "";
    if (feature.properties) {
        if (feature.properties.timestamp) {
            popupContent += "<p><b>" + feature.properties.timestamp + "</b></p>";
        }
        if (feature.properties.speed) {
            popupContent += "<p>" + 3.6 * feature.properties.speed + " km/h</p>";
        }
    }
    layer.bindPopup(popupContent);
}

async function fetchDataJSON(dateStart, dateEnd) {
    const response = await fetch(baseUrl + "/api/query?start=" + dateStart.format() + "&end=" + dateEnd.format());
    const content = await response.json();
    return content;
}

function drawGeoJSON(json) {
    for (const device_name in json.devices) {
        let json_device = json.devices[device_name];
        json_device.sort((obj1, obj2) => moment(obj1[5]) - moment(obj2[5]));
        let times = json_device.map((obj) => moment(obj[5]));
        let timestamps = json_device.map((obj) => {
            let date = moment(obj[5]);
            return date.hour().toString() + ":" + date.minute().toString();
        });
        let time_max = Math.max(...times);
        let time_min = Math.min(...times);

        let battery_vals = json_device.map((obj) => {
            return {
                x: moment(obj[5]),
                y: obj[3],
            };
        });
        let battery_states = json_device.map((obj) => json.states[obj[4]]);

        let coords = json_device.map((d) => [d[1], d[0]]);
        let max_coords = coords.reduce(function(a, b) {
            return [Math.max(a[0], b[0]), Math.max(a[1], b[1])];
        }, [-Infinity, -Infinity]);
        let min_coords = coords.reduce(function(a, b) {
            return [Math.min(a[0], b[0]), Math.min(a[1], b[1])];
        }, [Infinity, Infinity]);
        map.setView([(max_coords[0] + min_coords[0]) / 2,
            (max_coords[1] + min_coords[1]) / 2
        ]);

        overlayLayer = L.geoJSON(makeGeoJSONFromAray(coords, times), {

            style: function(feature) {
                return feature.properties && feature.properties.style;
            },
            onEachFeature: onEachFeature,
            pointToLayer: function(feature, latlng) {
                let dtime = Date.parse(feature.properties.datetime);
                let percent = (dtime - time_min) / (time_max - time_min);
                let color = "rgb(" + (percent * 255) + ", " + (120 * percent) + ", " + ((1 - percent) * 255) + ")";
                return L.circleMarker(latlng, {
                    radius: 3,
                    fillColor: color,
                    weight: 0,
                    opacity: 0.5,
                    fillOpacity: 0.5
                });
            }
        }).addTo(map);
        map.fitBounds(overlayLayer.getBounds());

        myChart.data.datasets[0].data = battery_vals;
        // myChart.data.datasets[1].data = battery_states;
        myChart.data.labels = timestamps;
        myChart.update();

        break;
    }

    // let line = L.polyline(coords).addTo(map);

}

function updateData(dateStart, dateEnd) {
    if (overlayLayer) {
        map.removeLayer(overlayLayer);
    }
    fetchDataJSON(dateStart, dateEnd)
        .then(drawGeoJSON)
        .catch(err => console.error(`Error fetching data: ${err.message}`));

}

function makeGeoJSONFromAray(coords, times) {
    return coords.map((d, t) => {
        return {
            "type": 'Feature',
            "geometry": {
                "type": 'Point',
                "coordinates": [d[1], d[0]],
            },
            "properties": {
                "datetime": times[t]
            },

        };
    });
}


updateData(dateStart, dateEnd);
